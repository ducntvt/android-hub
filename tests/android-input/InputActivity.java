package local.androidhub.inputqa;

import android.app.Activity;
import android.os.Bundle;
import android.graphics.Canvas;
import android.graphics.Color;
import android.graphics.Paint;
import android.view.MotionEvent;
import android.view.KeyEvent;
import android.view.View;

/** Offline fixture: three independent scroll targets plus raw event coordinates. */
public final class InputActivity extends Activity {
    @Override public void onCreate(Bundle state) {
        super.onCreate(state);
        setContentView(new Surface());
    }
    final class Surface extends View {
        final Paint paint = new Paint(Paint.ANTI_ALIAS_FLAG);
        final float[] horizontal = new float[3], vertical = new float[3];
        String event = "No wheel received";
        int touches = 0, releases = 0, cancellations = 0, moves = 0;
        int keyDowns = 0, keyUps = 0;
        boolean held = false;
        String touchEvent = "NONE";
        Surface() {
            super(InputActivity.this);
            setBackgroundColor(Color.WHITE);
            setFocusableInTouchMode(true);
            requestFocus();
        }
        void label(Canvas canvas, String value, float x, float y, int size) {
            paint.setColor(Color.BLACK); paint.setTextSize(size); canvas.drawText(value, x, y, paint);
        }
        @Override protected void onDraw(Canvas canvas) {
            label(canvas, "HUB INPUT QA", 20, 45, 30);
            label(canvas, event, 20, 80, 18);
            label(canvas, "Down " + touches + "  Up " + releases + "  Cancel " + cancellations, 20, 108, 18);
            label(canvas, "Held " + held + "  Last " + touchEvent + "  Moves " + moves, 20, 132, 18);
            float height = (getHeight() - 140f) / 3;
            for (int i = 0; i < 3; i++) {
                float top = 140 + height * i;
                paint.setColor(new int[]{0xffdbeafe,0xffdcfce7,0xfffef3c7}[i]);
                canvas.drawRect(0, top, getWidth(), top + height - 5, paint);
                label(canvas, "ZONE " + (i + 1), 20, top + 40, 28);
                label(canvas, String.format(java.util.Locale.ROOT, "H %.2f   V %.2f", horizontal[i], vertical[i]), 20, top + 75, 24);
                canvas.save(); canvas.clipRect(0, top + 90, getWidth(), top + height - 5);
                for (int row = -20; row < 30; row++) {
                    label(canvas, "Row " + row + "    horizontal marker", 30 + horizontal[i] * 15,
                        top + 120 + row * 45 + vertical[i] * 15, 22);
                }
                canvas.restore();
            }
        }
        @Override public boolean onGenericMotionEvent(MotionEvent e) {
            if (e.getAction() != MotionEvent.ACTION_SCROLL) return super.onGenericMotionEvent(e);
            float h = e.getAxisValue(MotionEvent.AXIS_HSCROLL), v = e.getAxisValue(MotionEvent.AXIS_VSCROLL);
            int zone = (int)((e.getY() - 140) / ((getHeight() - 140f) / 3));
            event = String.format(java.util.Locale.ROOT, "x %.0f y %.0f H %.2f V %.2f", e.getX(), e.getY(), h, v);
            if (e.getY() >= 140 && zone >= 0 && zone < 3) { horizontal[zone] += h; vertical[zone] += v; }
            invalidate(); return true;
        }
        @Override public boolean onTouchEvent(MotionEvent e) {
            switch (e.getActionMasked()) {
                case MotionEvent.ACTION_DOWN:
                    requestFocus();
                    touches++; held = true; touchEvent = "DOWN"; break;
                case MotionEvent.ACTION_MOVE:
                    moves++; touchEvent = "MOVE"; break;
                case MotionEvent.ACTION_UP:
                    releases++; held = false; touchEvent = "UP"; break;
                case MotionEvent.ACTION_CANCEL:
                    cancellations++; held = false; touchEvent = "CANCEL"; break;
                default:
                    break;
            }
            invalidate();
            return true;
        }
        @Override public boolean onKeyDown(int code, KeyEvent e) {
            if (code < KeyEvent.KEYCODE_A || code > KeyEvent.KEYCODE_Z) return super.onKeyDown(code, e);
            keyDowns++;
            event = "Key " + KeyEvent.keyCodeToString(code) + " Down " + keyDowns + " Up " + keyUps;
            invalidate();
            return true;
        }
        @Override public boolean onKeyUp(int code, KeyEvent e) {
            if (code < KeyEvent.KEYCODE_A || code > KeyEvent.KEYCODE_Z) return super.onKeyUp(code, e);
            keyUps++;
            event = "Key " + KeyEvent.keyCodeToString(code) + " Down " + keyDowns + " Up " + keyUps;
            invalidate();
            return true;
        }
    }
}
