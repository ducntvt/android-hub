package local.androidhub;

import android.os.SystemClock;
import android.view.InputDevice;
import android.view.InputEvent;
import android.view.MotionEvent;
import android.view.Display;
import android.graphics.Point;
import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.io.FileReader;
import java.lang.reflect.Method;
import java.util.Locale;

/** One adb shell session; stdin EOF terminates it. Never opens a network socket. */
public final class ScrollBridge {
    private static Class<?> inputManagerClass() throws ClassNotFoundException {
        // Android 14+ moved the shell-accessible singleton to InputManagerGlobal.
        // Android 12/13 expose the same injection contract on InputManager.
        try {
            return Class.forName("android.hardware.input.InputManagerGlobal");
        } catch (ClassNotFoundException unavailableOnOlderAndroid) {
            return Class.forName("android.hardware.input.InputManager");
        }
    }

    public static void main(String[] args) throws Exception {
        if (args.length != 1) throw new IllegalArgumentException("boot identity required");
        try (BufferedReader boot = new BufferedReader(new FileReader("/proc/sys/kernel/random/boot_id"))) {
            if (!args[0].equals(boot.readLine())) throw new IllegalStateException("guest restarted");
        }
        Class<?> managerClass = inputManagerClass();
        Object manager = managerClass.getMethod("getInstance").invoke(null);
        Method inject = managerClass.getMethod("injectInputEvent", InputEvent.class, int.class);
        Class<?> displaysClass = Class.forName("android.hardware.display.DisplayManagerGlobal");
        Object displays = displaysClass.getMethod("getInstance").invoke(null);
        Display display = (Display) displaysClass.getMethod("getRealDisplay", int.class).invoke(displays, 0);
        BufferedReader input = new BufferedReader(new InputStreamReader(System.in));
        System.out.println("READY 1");
        System.out.flush();
        String line;
        while ((line = input.readLine()) != null) {
            if (line.length() > 160) throw new IllegalArgumentException("message too long");
            String[] values = line.split(" ");
            if (values.length != 5) throw new IllegalArgumentException("invalid message");
            long sequence = Long.parseLong(values[0]);
            int x = Integer.parseInt(values[1]);
            int y = Integer.parseInt(values[2]);
            float horizontal = Float.parseFloat(values[3]);
            float vertical = Float.parseFloat(values[4]);
            if (sequence < 0 || x < 0 || y < 0 || x > 16384 || y > 16384
                    || !Float.isFinite(horizontal) || !Float.isFinite(vertical)
                    || Math.abs(horizontal) > 10 || Math.abs(vertical) > 10) {
                throw new IllegalArgumentException("out of range");
            }
            MotionEvent.PointerProperties property = new MotionEvent.PointerProperties();
            property.id = 0;
            property.toolType = MotionEvent.TOOL_TYPE_MOUSE;
            MotionEvent.PointerCoords coordinate = new MotionEvent.PointerCoords();
            coordinate.x = x;
            coordinate.y = y;
            // Hub sends physical panel coordinates, like gRPC touch. Framework
            // scroll injection instead needs logical coordinates of the active
            // Android display, whose rotation can differ from the sensors when
            // an app locks its orientation.
            Point size = new Point();
            display.getRealSize(size);
            float originalHorizontal = horizontal;
            switch (display.getRotation()) {
                case 1:
                    coordinate.x = y;
                    coordinate.y = size.y - 1 - x;
                    horizontal = vertical;
                    vertical = -originalHorizontal;
                    break;
                case 2:
                    coordinate.x = size.x - 1 - x;
                    coordinate.y = size.y - 1 - y;
                    horizontal = -horizontal;
                    vertical = -vertical;
                    break;
                case 3:
                    coordinate.x = size.x - 1 - y;
                    coordinate.y = x;
                    horizontal = -vertical;
                    vertical = originalHorizontal;
                    break;
                default:
                    break;
            }
            coordinate.setAxisValue(MotionEvent.AXIS_HSCROLL, horizontal);
            coordinate.setAxisValue(MotionEvent.AXIS_VSCROLL, vertical);
            long now = SystemClock.uptimeMillis();
            MotionEvent event = MotionEvent.obtain(now, now, MotionEvent.ACTION_SCROLL, 1,
                    new MotionEvent.PointerProperties[]{property},
                    new MotionEvent.PointerCoords[]{coordinate}, 0, 0, 1f, 1f,
                    0, 0, InputDevice.SOURCE_MOUSE, 0);
            try {
                // WAIT_FOR_RESULT: acknowledge framework acceptance, not just enqueue.
                if (!Boolean.TRUE.equals(inject.invoke(manager, event, 1))) {
                    throw new IllegalStateException("scroll rejected");
                }
            } finally {
                event.recycle();
            }
            System.out.printf(Locale.ROOT, "OK %d%n", sequence);
            System.out.flush();
        }
    }
}
