public class Integers2 {
    public static void main(String[] args) {
        System.out.println("Byte");
        testByteArithmetic();
        System.out.println("Short");
        testShortArithmetic();
        System.out.println("Char");
        testCharArithmetic();
    }

    public static void testByteArithmetic() {
        byte b1 = 127;
        byte b2 = -128;
        byte b3 = (byte) (b1 + b2);
        byte b4 = (byte) (b1 - b2);
        byte b5 = (byte) (b1 * b2);

        System.out.println(b1);
        System.out.println(b2);
        System.out.println(b3);
        System.out.println(b4);
        System.out.println(b5);
    }

    public static void testShortArithmetic() {
        short s1 = 32767;
        short s2 = -32768;
        short s3 = (short) (s1 + s2);
        short s4 = (short) (s1 - s2);
        short s5 = (short) (s1 * s2);

        System.out.println(s1);
        System.out.println(s2);
        System.out.println(s3);
        System.out.println(s4);
        System.out.println(s5);
    }

    public static void testCharArithmetic() {
        char c1 = 65535;
        char c2 = 0;
        char c3 = (char) (c1 + c2);
        char c4 = (char) (c1 - c2);
        char c5 = (char) (c1 * c2);

        System.out.println((int) c1);
        System.out.println((int) c2);
        System.out.println((int) c3);
        System.out.println((int) c4);
        System.out.println((int) c5);
    }
}
