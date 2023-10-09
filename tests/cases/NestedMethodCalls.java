public class NestedMethodCalls {
    public static void main(String[] args) {
        System.out.println(nestedAdd(3, 4)); // Should print 7
        System.out.println(nestedMultiplyThenAdd(3, 4, 5)); // Should print 17 (3*4 + 5)
    }

    public static int nestedAdd(int a, int b) {
        return add(add(a, b), add(a, b)); // (3+4) + (3+4) = 7 + 7 = 14
    }

    public static int add(int a, int b) {
        return a + b;
    }

    public static int nestedMultiplyThenAdd(int a, int b, int c) {
        return add(multiply(a, b), c); // (3 * 4) + 5 = 12 + 5 = 17
    }

    public static int multiply(int a, int b) {
        return a * b;
    }
}
