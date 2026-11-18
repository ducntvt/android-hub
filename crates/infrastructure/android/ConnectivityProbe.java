package local.androidhub;

import java.io.BufferedReader;
import java.io.FileReader;
import java.io.IOException;
import java.net.InetAddress;
import java.net.NetworkInterface;
import java.net.SocketTimeoutException;
import java.net.URL;
import java.net.UnknownHostException;
import java.util.Enumeration;
import java.util.concurrent.Callable;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;
import javax.net.ssl.HttpsURLConnection;
import javax.net.ssl.SSLException;

/** One user-requested guest probe. Emits a fixed vocabulary, never addresses or bodies. */
public final class ConnectivityProbe {
    static final String HOST = "www.google.com";
    static final String ENDPOINT = "https://www.google.com/generate_204";

    static String classify(Throwable error) {
        if (error instanceof SSLException) return "TLS";
        if (error instanceof SocketTimeoutException) return "TIMEOUT";
        if (error instanceof UnknownHostException) return "DNS";
        if (error instanceof SecurityException) return "UNAVAILABLE";
        if (error instanceof IOException) return "IO";
        return "UNAVAILABLE";
    }

    static String bounded(Callable<String> action, long milliseconds) {
        ExecutorService worker = Executors.newSingleThreadExecutor(r -> {
            Thread thread = new Thread(r, "hub-network-step");
            thread.setDaemon(true);
            return thread;
        });
        Future<String> future = worker.submit(action);
        try {
            return future.get(milliseconds, TimeUnit.MILLISECONDS);
        } catch (TimeoutException error) {
            return "TIMEOUT";
        } catch (ExecutionException error) {
            return classify(error.getCause());
        } catch (InterruptedException error) {
            Thread.currentThread().interrupt();
            return "UNAVAILABLE";
        } finally {
            future.cancel(true);
            worker.shutdownNow();
        }
    }

    static String interfaces() throws Exception {
        Enumeration<NetworkInterface> interfaces = NetworkInterface.getNetworkInterfaces();
        if (interfaces == null) return "UNAVAILABLE";
        while (interfaces.hasMoreElements()) {
            NetworkInterface network = interfaces.nextElement();
            if (!network.isUp() || network.isLoopback()) continue;
            Enumeration<InetAddress> addresses = network.getInetAddresses();
            while (addresses.hasMoreElements()) {
                InetAddress address = addresses.nextElement();
                if (!address.isLoopbackAddress() && !address.isLinkLocalAddress()
                        && !address.isAnyLocalAddress()) return "OK";
            }
        }
        return "NO_ADDRESS";
    }

    static String https() throws Exception {
        HttpsURLConnection connection = (HttpsURLConnection) new URL(ENDPOINT).openConnection();
        try {
            // Keep Android's default certificate trust and hostname verification.
            connection.setConnectTimeout(4000);
            connection.setReadTimeout(4000);
            connection.setInstanceFollowRedirects(false);
            connection.setUseCaches(false);
            connection.setAllowUserInteraction(false);
            connection.setRequestProperty("User-Agent", "AndroidHub-Connectivity/1");
            int status = connection.getResponseCode();
            return status == 204 ? "OK" : (status >= 100 && status <= 599
                    ? "HTTP_" + status : "UNAVAILABLE");
        } finally {
            // Do not read/store response bodies, headers, cookies or account state.
            connection.disconnect();
        }
    }

    private static void daemon(String name, Runnable action) {
        Thread thread = new Thread(action, name);
        thread.setDaemon(true);
        thread.start();
    }

    public static void main(String[] args) throws Exception {
        if (args.length != 1) System.exit(2);
        try (BufferedReader boot = new BufferedReader(new FileReader("/proc/sys/kernel/random/boot_id"))) {
            if (!args[0].equals(boot.readLine())) System.exit(3);
        }
        // Closing the adb stream cancels the guest worker too. The watchdog is
        // independent of DNS/native networking, which may ignore interruption.
        daemon("hub-network-disconnect", () -> {
            try { while (System.in.read() != -1) { } } catch (IOException ignored) { }
            System.exit(4);
        });
        daemon("hub-network-deadline", () -> {
            try { Thread.sleep(20000); } catch (InterruptedException ignored) { }
            System.exit(5);
        });
        System.out.println("HUBNET1 " + args[0]);
        System.out.println("INTERFACE " + bounded(ConnectivityProbe::interfaces, 1000));
        System.out.println("DNS " + bounded(() ->
                InetAddress.getAllByName(HOST).length > 0 ? "OK" : "DNS", 4000));
        System.out.println("HTTPS " + bounded(ConnectivityProbe::https, 9000));
        System.out.println("DONE");
        System.out.flush();
        System.exit(0);
    }
}
