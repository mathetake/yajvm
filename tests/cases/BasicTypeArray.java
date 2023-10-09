public class BasicTypeArray {
    public static void main(String[] args) {
        intArray();
        booleanArray();
        byteArray();
        shortArray();
        charArray();
        longArray();
        floatArray();
        doubleArray();
    }

    public static void intArray() {
        int[] arr = new int[3];
        arr[0] = 1;
        arr[1] = 2;
        arr[2] = 3;

        printIntArray(arr);
    }

    public static void booleanArray() {
        boolean[] arr = new boolean[3];
        arr[0] = true;
        arr[1] = false;
        arr[2] = true;

        printBooleanArray(arr);
    }

    public static void byteArray() {
        byte[] arr = new byte[3];
        arr[0] = 10;
        arr[1] = 20;
        arr[2] = 30;

        printByteArray(arr);
    }

    public static void shortArray() {
        short[] arr = new short[3];
        arr[0] = 100;
        arr[1] = 200;
        arr[2] = 300;

        printShortArray(arr);
    }

    public static void charArray() {
        char[] arr = new char[3];
        arr[0] = 'A';
        arr[1] = 'B';
        arr[2] = 'C';

        printCharArray(arr);
    }

    public static void longArray() {
        long[] arr = new long[3];
        arr[0] = 1000L;
        arr[1] = 2000L;
        arr[2] = 3000L;

        printLongArray(arr);
    }

    public static void floatArray() {
        float[] arr = new float[3];
        arr[0] = 1.1f;
        arr[1] = 2.2f;
        arr[2] = 3.3f;

        printFloatArray(arr);
    }

    public static void doubleArray() {
        double[] arr = new double[3];
        arr[0] = 1.11;
        arr[1] = 2.22;
        arr[2] = 3.33;

        printDoubleArray(arr);
    }

    public static void printIntArray(int[] arr) {
        System.out.println(arr[0]);
        System.out.println(arr[1]);
        System.out.println(arr[2]);
    }

    public static void printBooleanArray(boolean[] arr) {
        System.out.println(arr[0]);
        System.out.println(arr[1]);
        System.out.println(arr[2]);
    }

    public static void printByteArray(byte[] arr) {
        System.out.println(arr[0]);
        System.out.println(arr[1]);
        System.out.println(arr[2]);
    }

    public static void printShortArray(short[] arr) {
        System.out.println(arr[0]);
        System.out.println(arr[1]);
        System.out.println(arr[2]);
    }

    public static void printCharArray(char[] arr) {
        System.out.println(arr[0]);
        System.out.println(arr[1]);
        System.out.println(arr[2]);
    }

    public static void printLongArray(long[] arr) {
        System.out.println(arr[0]);
        System.out.println(arr[1]);
        System.out.println(arr[2]);
    }

    public static void printFloatArray(float[] arr) {
        System.out.println(arr[0]);
        System.out.println(arr[1]);
        System.out.println(arr[2]);
    }

    public static void printDoubleArray(double[] arr) {
        System.out.println(arr[0]);
        System.out.println(arr[1]);
        System.out.println(arr[2]);
    }
}
