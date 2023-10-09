public class MutualRecursion {
    public static void main(String[] args) {
        int result = foo(5);
        System.out.println(result);
    }

    public static int foo(int n) {
        if (n <= 0) return 1;
        return n - bar(n - 1);
    }

    public static int bar(int n) {
        if (n <= 0) return 0;
        return n - foo(n - 1);
    }
}
