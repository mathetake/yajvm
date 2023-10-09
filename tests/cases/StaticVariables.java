public class StaticVariables {
    static boolean boolVar = true;
    static byte byteVar;
    static short shortVar;
    static char charVar;
    static int intVar;
    static long longVar;
    static float floatVar;
    static double doubleVar;

    public static void main(String[] args) {
        testStaticBoolean();
        testStaticByte();
        testStaticShort();
        testStaticChar();
        testStaticInt();
        testStaticLong();
        testStaticFloat();
        testStaticDouble();
    }

    public static void testStaticBoolean() {
        boolVar = true;
        System.out.println(boolVar ? 1 : 0);  // should print 1
    }

    public static void testStaticByte() {
        byteVar = 42;
        System.out.println(byteVar);  // should print 42
    }

    public static void testStaticShort() {
        shortVar = 32767;
        System.out.println(shortVar);  // should print 32767
    }

    public static void testStaticChar() {
        charVar = 'A';
        System.out.println((int) charVar);  // should print 65
    }

    public static void testStaticInt() {
        intVar = Integer.MAX_VALUE;
        System.out.println(intVar);  // should print 2147483647
    }

    public static void testStaticLong() {
        longVar = Long.MAX_VALUE;
        System.out.println(longVar);  // should print 9223372036854775807
    }

    public static void testStaticFloat() {
        floatVar = 3.14f;
        System.out.println(floatVar);  // should print 3.14
    }

    public static void testStaticDouble() {
        doubleVar = 2.71828;
        System.out.println(doubleVar);  // should print 2.71828
    }
}
