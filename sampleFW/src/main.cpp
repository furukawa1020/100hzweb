#include <Arduino.h>
#include <M5unified.h>
#include <Wire.h>
#include <math.h>

#include "MAX30100.h"

// Sampling is tightly related to the dynamic range of the ADC.
// refer to the datasheet for further info
#define SAMPLING_RATE MAX30100_SAMPRATE_100HZ
// #define SAMPLING_RATE MAX30100_SAMPRATE_50HZ

// The LEDs currents must be set to a level that avoids clipping and maximises
// the dynamic range
#define IR_LED_CURRENT MAX30100_LED_CURR_50MA
#define RED_LED_CURRENT MAX30100_LED_CURR_27_1MA

// The pulse width of the LEDs driving determines the resolution of
// the ADC (which is a Sigma-Delta).
// set HIGHRES_MODE to true only when setting PULSE_WIDTH to
// MAX30100_SPC_PW_1600US_16BITS
#define PULSE_WIDTH MAX30100_SPC_PW_1600US_16BITS
#define HIGHRES_MODE true

// Instantiate a MAX30100 sensor class
MAX30100 sensor;

// --- フィルタ設定パラメータ (filterESP32.cから移植) ---
const int SAMPLE_RATE = 100; // 100Hz
const int WINDOW_SIZE = 150; // 1.5秒間のウィンドウ (相関・標準偏差計算用)
const int FILTER_ORDER =
    4; // フィルタ次数 + 1 (係数の数) -> 4次フィルタなら係数は5個

// --- フィルタ係数 (Pythonで計算した値をここに設定) ---
// Bandpass Filter (0.5 - 5.0 Hz), fs=100Hz, order=4
const float B_BPF[] = {0.0006760592, 0.0000000000, -0.0013521185, 0.0000000000,
                       0.0006760592};
const float A_BPF[] = {1.0000000000, -3.6186414773, 4.9356346257, -3.0031317094,
                       0.6865261313};

// Lowpass Filter (0.5 Hz) for Baseline, fs=100Hz, order=4
const float B_LPF[] = {0.0000000570, 0.0000002281, 0.0000003422, 0.0000002281,
                       0.0000000570};
const float A_LPF[] = {1.0000000000, -3.9370720516, 5.8118005697, -3.8119614450,
                       0.9372338394};

// --- PPGプロセッサクラス (filterESP32.cから移植) ---
class PPGProcessor {
private:
  // フィルタの状態変数 (直近の入出力を保持)
  float x_red[FILTER_ORDER + 1] = {0};
  float y_red[FILTER_ORDER + 1] = {0};
  float x_ir[FILTER_ORDER + 1] = {0};
  float y_ir[FILTER_ORDER + 1] = {0};

  // ベースライン用フィルタ状態変数
  float x_base[FILTER_ORDER + 1] = {0};
  float y_base[FILTER_ORDER + 1] = {0};

  // リングバッファ (判定計算用)
  float bufFiltRed[WINDOW_SIZE];
  float bufFiltIR[WINDOW_SIZE];
  float bufRawIR[WINDOW_SIZE]; // 接触判定用
  int bufIndex = 0;
  bool bufferFilled = false;

  // 前回のベースライン値 (傾き計算用)
  float prevBaseline = 0;

  // IIRフィルタ計算関数
  float applyIIR(float input, float *x, float *y, const float *b,
                 const float *a) {
    // シフト処理
    for (int i = FILTER_ORDER; i > 0; i--) {
      x[i] = x[i - 1];
      y[i] = y[i - 1];
    }
    x[0] = input;

    // 差分方程式: y[0] = (b[0]x[0] + ... + b[N]x[N]) - (a[1]y[1] + ... +
    // a[N]y[N]) a[0]は通常1.0なので省略
    float val = 0;
    for (int i = 0; i <= FILTER_ORDER; i++) {
      val += b[i] * x[i];
    }
    for (int i = 1; i <= FILTER_ORDER; i++) {
      val -= a[i] * y[i];
    }
    y[0] = val;
    return val;
  }

public:
  struct PPGResult {
    float filtRed;
    float filtIR;
    int qualityFlag; // 1: Valid, 0: Invalid
  };

  PPGProcessor() {
    // バッファ初期化
    for (int i = 0; i < WINDOW_SIZE; i++) {
      bufFiltRed[i] = 0;
      bufFiltIR[i] = 0;
      bufRawIR[i] = 0;
    }
  }

  // 毎サンプリング呼び出す関数
  PPGResult update(float rawRed, float rawIR) {
    PPGResult res;

    // 1. バンドパスフィルタ適用 (脈波抽出)
    res.filtRed = applyIIR(rawRed, x_red, y_red, B_BPF, A_BPF);
    res.filtIR = applyIIR(rawIR, x_ir, y_ir, B_BPF, A_BPF);

    // 2. ローパスフィルタ適用 (ベースライン抽出 for IR)
    float baselineIR = applyIIR(rawIR, x_base, y_base, B_LPF, A_LPF);

    // 3. バッファにデータを追加 (リングバッファ)
    bufFiltRed[bufIndex] = res.filtRed;
    bufFiltIR[bufIndex] = res.filtIR;
    bufRawIR[bufIndex] = rawIR;

    // 4. ベースライン変動チェック (傾き)
    float slope = abs(baselineIR - prevBaseline); // 簡易微分
    prevBaseline = baselineIR;

    // --- 品質判定ロジック ---
    // まだバッファが埋まっていない場合は判定しない(0)
    if (!bufferFilled && bufIndex < WINDOW_SIZE - 1) {
      res.qualityFlag = 0;
    } else {
      // リングバッファが一周したらフラグを立てる
      if (bufIndex == WINDOW_SIZE - 1)
        bufferFilled = true;

      // 統計計算 (平均、標準偏差、共分散)
      float sumRed = 0, sumIR = 0;
      float sumRedSq = 0, sumIRSq = 0;
      float sumCross = 0;
      float minRawIR = 1000000; // Rawデータの最小値チェック用

      for (int i = 0; i < WINDOW_SIZE; i++) {
        float r = bufFiltRed[i];
        float ir = bufFiltIR[i];
        float raw = bufRawIR[i];

        sumRed += r;
        sumIR += ir;
        sumRedSq += r * r;
        sumIRSq += ir * ir;
        sumCross += r * ir;
        if (raw < minRawIR)
          minRawIR = raw;
      }

      float meanRed = sumRed / WINDOW_SIZE;
      float meanIR = sumIR / WINDOW_SIZE;

      // 分散・共分散
      // Var(X) = E[X^2] - (E[X])^2
      float varRed = (sumRedSq / WINDOW_SIZE) - (meanRed * meanRed);
      float varIR = (sumIRSq / WINDOW_SIZE) - (meanIR * meanIR);
      float cov = (sumCross / WINDOW_SIZE) - (meanRed * meanIR);

      float stdRed = sqrt(varRed > 0 ? varRed : 0);
      float stdIR = sqrt(varIR > 0 ? varIR : 0);

      // 相関係数
      float correlation = 0;
      if (stdRed > 0 && stdIR > 0) {
        correlation = cov / (stdRed * stdIR);
      }

      // 判定条件 (Pythonコード準拠)
      // 1. 相関 > 0.8
      // 2. 振幅(stdIR) > 5.0
      // 3. Raw値(接触) > 10000 (ここではバッファ内の最小値で判定)
      // 4. ベースライン傾き < 10.0 (現在の傾きを使用)

      bool isCorrOK = correlation > 0.8;
      bool isAmpOK = stdIR > 1.3;
      bool isContactOK = minRawIR > 10000;
      bool isStable =
          slope < 10.0; // 閾値はサンプリングレートやゲインによるので要調整
      //            if (isCorrOK && isAmpOK && isContactOK && isStable) {
      if (isCorrOK && isAmpOK && isContactOK) {
        res.qualityFlag = 1;
      } else {
        res.qualityFlag = 0;
      }
      //            printf("%d : %d %d (%.2f) %d %d(%.2f / %.2f %.2f)\n",
      //            res.qualityFlag, isCorrOK, isAmpOK, stdIR, isContactOK,
      //            isStable, slope, baselineIR, prevBaseline);
    }

    // インデックス更新
    bufIndex++;
    if (bufIndex >= WINDOW_SIZE)
      bufIndex = 0;

    return res;
  }

  // リセット関数 (ボタン押下時用)
  void reset() {
    for (int i = 0; i <= FILTER_ORDER; i++) {
      x_red[i] = 0;
      y_red[i] = 0;
      x_ir[i] = 0;
      y_ir[i] = 0;
      x_base[i] = 0;
      y_base[i] = 0;
    }
    for (int i = 0; i < WINDOW_SIZE; i++) {
      bufFiltRed[i] = 0;
      bufFiltIR[i] = 0;
      bufRawIR[i] = 0;
    }
    bufIndex = 0;
    bufferFilled = false;
    prevBaseline = 0;
  }
};

// PPGプロセッサのインスタンス
PPGProcessor ppgProcessor;

// display: 128 x 128 (M5Stack ATOMS3)
#define X 128
#define Y 128
#define N 128
#define Nbit 7

uint16_t val_red[N], val_ir[N];
uint8_t px = 0;

// リングバッファ: 60秒分（6000サンプル、100Hz）のデータを保存
// メモリ制約のため、実際のサンプリングレートに合わせて6000サンプルに設定
#define BUFFER_SIZE 6000
struct DataSample {
  uint16_t red;       // 生データ (Red)
  uint16_t ir;        // 生データ (IR)
  float filtRed;      // フィルタ後のデータ (Red)
  float filtIR;       // フィルタ後のデータ (IR)
  uint16_t timestamp; // タイムスタンプ（ms、65535ms = 65秒まで対応可能）
};
DataSample dataBuffer[BUFFER_SIZE];
uint32_t bufferIndex = 0;
uint32_t sampleCount = 0; // 保存されたサンプル数
uint32_t timeBase =
    0; // タイムスタンプの基準時刻（起動時または前回のUART転送終了時）
int currentQualityFlag = 0; // 現在の品質フラグ (グラフ描画用)

#define Navg_base 100
#define Navg_data 10
#define Navg_slope 10
int data0 = 0;
uint8_t fRamp = 0, fRamp0 = 0;
int slope[Navg_slope];
uint8_t pSlope = 0;
uint16_t tNP = 0, tPN = 0, tPN0 = 0, periodPN = 0, periodNP = 0;
uint8_t fValid = 0;
uint32_t sum_base = 0, sum_data = 0;
long sum_slope = 0;

void setup() {
  // I2CピンをGroveコネクタのGPIO1(SCL)とGPIO2(SDA)に設定
  Wire.setPins(2, 1); // SDA=GPIO2, SCL=GPIO1
  Wire.begin();

  M5.begin();

  if (!sensor.begin()) {
    printf("fail\n");
    for (;;)
      ;
  } else {
    printf("ok\n");
  }

  // Set up the wanted parameters
  sensor.setMode(MAX30100_MODE_SPO2_HR);
  sensor.setLedsCurrent(IR_LED_CURRENT, RED_LED_CURRENT);
  sensor.setLedsPulseWidth(PULSE_WIDTH);
  sensor.setSamplingRate(SAMPLING_RATE);
  sensor.setHighresModeEnabled(HIGHRES_MODE);
  M5.Lcd.clear();
  for (uint16_t x = 0; x < N; x++) {
    val_red[x] = 0;
    val_ir[x] = 0;
  }

  // リングバッファの初期化
  for (uint32_t i = 0; i < BUFFER_SIZE; i++) {
    dataBuffer[i].red = 0;
    dataBuffer[i].ir = 0;
    dataBuffer[i].filtRed = 0;
    dataBuffer[i].filtIR = 0;
    dataBuffer[i].timestamp = 0;
  }
  bufferIndex = 0;
  sampleCount = 0;
  currentQualityFlag = 0;

  // 基準時刻を設定（起動時をt=0とする）
  timeBase = millis();

  printf("Data buffer initialized. Press button to export data.\n");
}

uint8_t f = 0;

void loop() {
  M5.update();

  // ボタン押下の検出（ボタンを押したらデータをUARTに出力）
  if (M5.BtnA.wasClicked()) {
    uint32_t maxCount;
    if (sampleCount < BUFFER_SIZE)
      maxCount = sampleCount;
    else
      maxCount = BUFFER_SIZE;
    printf("# Index,Timestamp,Red,IR,FiltRed,FiltIR (%lu)\n", maxCount);

    for (uint32_t i = 0; i < maxCount; i++) {
      printf("%lu,%u,%u,%u,%.2f,%.2f\n", i, dataBuffer[i].timestamp,
             dataBuffer[i].red, dataBuffer[i].ir, dataBuffer[i].filtRed,
             dataBuffer[i].filtIR);
    }

    // データ転送終了後、基準時刻をリセットしてバッファをクリア
    timeBase = millis();
    bufferIndex = 0;
    sampleCount = 0;

    // 画面描画用のバッファと状態変数もリセット
    px = 0;
    for (uint16_t x = 0; x < N; x++) {
      val_red[x] = 0;
      val_ir[x] = 0;
    }
    data0 = 0;
    fRamp = 0;
    fRamp0 = 0;
    f = 0;
    pSlope = 0;
    tNP = 0;
    tPN = 0;
    tPN0 = 0;
    periodPN = 0;
    periodNP = 0;
    fValid = 0;
    currentQualityFlag = 0;

    // PPGプロセッサをリセット
    ppgProcessor.reset();

    // 画面をクリアして新しい計測を開始
    M5.Lcd.clear();

    printf("Buffer cleared. New measurement started from t=0.\n");
    sensor.resetFifo();
  }

  uint16_t ir, red;
  sensor.update();

  while (sensor.getRawValues(&ir, &red)) {
    val_red[px] = red;
    val_ir[px] = ir;

    // PPGプロセッサでフィルタ処理と品質判定を実行
    PPGProcessor::PPGResult ppgResult =
        ppgProcessor.update((float)red, (float)ir);
    currentQualityFlag = ppgResult.qualityFlag;

    // リングバッファにデータを保存（生データとフィルタ後のデータの両方）
    dataBuffer[bufferIndex].red = red;
    dataBuffer[bufferIndex].ir = ir;
    dataBuffer[bufferIndex].filtRed = ppgResult.filtRed;
    dataBuffer[bufferIndex].filtIR = ppgResult.filtIR;
    uint32_t elapsed = millis() - timeBase;
    dataBuffer[bufferIndex].timestamp =
        (elapsed > 65535) ? 65535 : (uint16_t)elapsed;
    bufferIndex = (bufferIndex + 1) % BUFFER_SIZE;
    if (sampleCount < BUFFER_SIZE)
      sampleCount++;

    // Real-time streaming for VASC-LAB Relay Server
    printf("S,%lu,%u,%u,%.2f,%.2f\n", sampleCount, ir, red, ppgResult.filtIR,
           ppgResult.filtRed);
    /*
      uint16_t px0;
        sum_base = 0;
        px0 = (X + px - Navg_base - 1) % X;
        for (uint8_t i = 0; i < Navg_base; i++)
          sum_base += val_red[(px0 + i) % X];
        sum_data = 0;
        px0 = (X + px - Navg_data - 1) % X;
        for (uint8_t i = 0; i < Navg_data; i++)
          sum_data += val_red[(px0 + i) % X];
        uint16_t base = sum_base / Navg_base;
        int data = sum_data / Navg_data - base;
        slope[pSlope] = data - data0;
        pSlope = (pSlope + 1) % Navg_slope;
        sum_slope = 0;
        for (uint8_t i = 0; i < Navg_slope; i++)
          sum_slope += slope[i];
        int avg_slope = sum_slope / Navg_slope;
        if (avg_slope > 0) fRamp = 1; else fRamp = 0;

        if (fRamp0 == 0 && fRamp == 1)
          tNP = millis();
        else if (fRamp0 == 1 && fRamp == 0){
            tPN = millis();
            periodPN = tPN - tNP;
            periodNP = tNP - tPN0;
            tPN0 = tPN;
        }
        fRamp0 = fRamp;
        data0 = data;
        if (periodPN > 400 && periodPN < 800 && periodNP > 100 && periodNP <
      300) fValid = 1; else fValid = 0;
        //printf("%d %d %d %d %d\n", red, base, data, periodPN, periodNP);
        //    printf(">red:%d\n", val_red[px]);
        //    printf(">max:%d\n", max_red);
        //    printf(">min:%d\n", min_red);
        */
    f++;
    if (f == 4) // draw every 4 samples
    {
      f = 0;
      uint16_t min_red = 65535, min_ir = 65535, max_red = 0, max_ir = 0;
      for (uint8_t x = 0; x < X; x++) {
        if (val_red[x] > max_red)
          max_red = val_red[x];
        if (val_red[x] < min_red)
          min_red = val_red[x];
        if (val_ir[x] > max_ir)
          max_ir = val_ir[x];
        if (val_ir[x] < min_ir)
          min_ir = val_ir[x];
      }
      uint16_t mag_red, mag_ir;
      if ((max_red - min_red) < Y)
        mag_red = 1;
      else
        mag_red = (max_red - min_red) / Y;
      if ((max_ir - min_ir) < Y)
        mag_ir = 1;
      else
        mag_ir = (max_ir - min_ir) / Y;
      uint8_t dx = px;
      int y_red, y_ir;
      for (uint8_t x = 0; x < X; x++) {
        y_red = (val_red[dx] - min_red) / mag_red;
        if (y_red > Y - 1)
          y_red = Y - 1;
        if (y_red < 0)
          y_red = 0;
        y_ir = (val_ir[dx] - min_ir) / mag_ir;
        if (y_ir > Y - 1)
          y_ir = Y - 1;
        if (y_ir < 0)
          y_ir = 0;
        M5.Lcd.drawFastVLine(x, 0, Y, BLACK);
        if (currentQualityFlag == 1)
          M5.Lcd.drawPixel(x, Y - 1 - y_red, GREEN);
        else
          M5.Lcd.drawPixel(x, Y - 1 - y_red, RED);
        dx = (dx + 1) % X;
      }
    }
    px = (px + 1) % X;
  }
}
