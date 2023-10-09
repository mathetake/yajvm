public class Integers {
    public static void main(String[] args) {
        System.out.println("Boolean");
        testBoolean();
        System.out.println("Byte");
        testByte();
        System.out.println("Short");
        testShort();
        System.out.println("Char");
        testChar();
    }

    public static void testBoolean() {
        boolean b1 = true;
        boolean b2 = false;
        boolean b3 = !b2;
        boolean b4 = b1 && b2;
        boolean b5 = b1 || b2;

        System.out.println(b1 ? 1 : 0);
        System.out.println(b2 ? 1 : 0);
        System.out.println(b3 ? 1 : 0);
        System.out.println(b4 ? 1 : 0);
        System.out.println(b5 ? 1 : 0);
    }

    public static void testByte() {
        byte b1 = 127;
        byte b2 = -128;
        byte b3 = (byte) (b1 + 1);  // Should wrap around
        byte b4 = (byte) (b2 - 1);  // Should wrap around
        byte b5 = (byte) (b1 * 2);  // Should wrap around

        System.out.println(b1);
        System.out.println(b2);
        System.out.println(b3);
        System.out.println(b4);
        System.out.println(b5);
    }

    public static void testShort() {
        short s1 = 32767;
        short s2 = -32768;
        short s3 = (short) (s1 + 1);  // Should wrap around
        short s4 = (short) (s2 - 1);  // Should wrap around
        short s5 = (short) (s1 * 2);  // Should wrap around

        System.out.println(s1);
        System.out.println(s2);
        System.out.println(s3);
        System.out.println(s4);
        System.out.println(s5);
    }

    public static void testChar() {
        char c1 = 0;
        char c2 = 65535;
        char c3 = (char) (c1 - 1);  // Should wrap around
        char c4 = (char) (c2 + 1);  // Should wrap around
        char c5 = (char) (c1 + 128);  // No wrap

        System.out.println((int)c1);
        System.out.println((int)c2);
        System.out.println((int)c3);
        System.out.println((int)c4);
        System.out.println((int)c5);
    }
}
