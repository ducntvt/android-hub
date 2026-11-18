package local.androidhub;

import java.io.IOException;
import java.net.SocketTimeoutException;
import java.net.UnknownHostException;
import javax.net.ssl.SSLHandshakeException;

/** No network access: validate deadlines and the redacted protocol vocabulary. */
public final class ConnectivityProbeTest {
    private static void equal(String expected, String actual) {
        if (!expected.equals(actual)) throw new AssertionError(expected + " != " + actual);
    }
    public static void main(String[] args) {
        equal("OK", ConnectivityProbe.bounded(() -> "OK", 500));
        equal("TLS", ConnectivityProbe.bounded(() -> { throw new SSLHandshakeException("secret cert"); }, 500));
        equal("DNS", ConnectivityProbe.bounded(() -> { throw new UnknownHostException("private host"); }, 500));
        equal("TIMEOUT", ConnectivityProbe.classify(new SocketTimeoutException("private endpoint")));
        equal("UNAVAILABLE", ConnectivityProbe.classify(new SecurityException("private context")));
        equal("IO", ConnectivityProbe.classify(new IOException("private address")));
        equal("UNAVAILABLE", ConnectivityProbe.classify(new IllegalStateException("secret")));
        long start = System.nanoTime();
        equal("TIMEOUT", ConnectivityProbe.bounded(() -> { Thread.sleep(30000); return "OK"; }, 30));
        if ((System.nanoTime() - start) / 1000000 > 2000) throw new AssertionError("deadline not enforced");
    }
}
