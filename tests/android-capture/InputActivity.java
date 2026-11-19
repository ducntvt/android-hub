package local.androidhub.inputqa;

import android.app.Activity;
import android.content.pm.ActivityInfo;
import android.graphics.Canvas;
import android.graphics.Color;
import android.graphics.Paint;
import android.os.Bundle;
import android.os.SystemClock;
import android.view.View;
import android.view.WindowManager;

/** Offline capture fixture, no permissions or phone data access. */
public final class InputActivity extends Activity {
    @Override public void onCreate(Bundle state) {
        super.onCreate(state);
        if (getIntent().getBooleanExtra("secure", false)) {
            getWindow().addFlags(WindowManager.LayoutParams.FLAG_SECURE);
        }
        setRequestedOrientation(getIntent().getBooleanExtra("landscape", false)
            ? ActivityInfo.SCREEN_ORIENTATION_LANDSCAPE : ActivityInfo.SCREEN_ORIENTATION_PORTRAIT);
        setContentView(new View(this) {
            final Paint paint = new Paint(Paint.ANTI_ALIAS_FLAG);
            final long start = SystemClock.uptimeMillis();
            @Override protected void onDraw(Canvas canvas) {
                long elapsed = SystemClock.uptimeMillis() - start;
                canvas.drawColor(Color.rgb(225, 239, 255));
                paint.setColor(Color.rgb(20, 40, 80));
                paint.setTextSize(42);
                canvas.drawText("Android Hub Capture QA", 30, 100, paint);
                canvas.drawText(getWidth() + " x " + getHeight(), 30, 165, paint);
                canvas.drawText("Elapsed: " + elapsed + " ms", 30, 230, paint);
                canvas.drawText(getIntent().getBooleanExtra("secure", false)
                    ? "SECURE: this content must not be mirrored" : "VISIBLE: capture is allowed", 30, 295, paint);
                paint.setColor(Color.rgb(40, 100, 220));
                float x = (elapsed % 2000) / 2000f * Math.max(1, getWidth() - 100);
                canvas.drawRect(x, getHeight() - 140, x + 100, getHeight() - 40, paint);
                if (elapsed < 120000) postInvalidateOnAnimation();
            }
        });
    }
}
