public class Numerics {
    public static void main(String[] args) {
        System.out.println("Integer");
        testIntegerArithmetic();
        System.out.println("Long");
        testLongArithmetic();
        System.out.println("Float");
        testFloatArithmetic();
        System.out.println("Double");
        testDoubleArithmetic();
    }

    public static void testIntegerArithmetic() {
        int i1 = Integer.MAX_VALUE;
        int i2 = Integer.MIN_VALUE;
        int i3 = i1 + i2;
        int i4 = i1 - i2;
        int i5 = i1 * 2;

        System.out.println(i1);
        System.out.println(i2);
        System.out.println(i3);
        System.out.println(i4);
        System.out.println(i5);
    }

    public static void testLongArithmetic() {
        long l1 = Long.MAX_VALUE;
        long l2 = Long.MIN_VALUE;
        long l3 = l1 + l2;
        long l4 = l1 - l2;
        long l5 = l1 * 2;

        System.out.println(l1);
        System.out.println(l2);
        System.out.println(l3);
        System.out.println(l4);
        System.out.println(l5);
    }

    public static void testFloatArithmetic() {
        float f1 = Float.MAX_VALUE;
        float f2 = Float.MIN_VALUE;
        float f3 = f1 + f2;
        float f4 = f1 - f2;
        float f5 = f1 * 2.0f;

        System.out.println(f1);
        System.out.println(f2);
        System.out.println(f3);
        System.out.println(f4);
        System.out.println(f5);
    }

    public static void testDoubleArithmetic() {
        double d1 = Double.MAX_VALUE;
        double d2 = Double.MIN_VALUE;
        double d3 = d1 + d2;
        double d4 = d1 - d2;
        double d5 = d1 * 2.0;

        System.out.println(d1);
        System.out.println(d2);
        System.out.println(d3);
        System.out.println(d4);
        System.out.println(d5);
    }
}
