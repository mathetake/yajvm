public class FcmpNan {
    public static void main(String[] args) {
        float a = 5.5f;
        float b = Float.NaN;

        int resultABG = fcmpg(a, b);
        int resultBAG = fcmpg(b, a);
        int resultBBG = fcmpg(b, b);

        int resultABL = fcmpl(a, b);
        int resultBAL = fcmpl(b, a);
        int resultBBL = fcmpl(b, b);

        System.out.println("fcmpg:");
        System.out.println(resultABG);  // Should print 1
        System.out.println(resultBAG);  // Should print 1
        System.out.println(resultBBG);  // Should print 1

        System.out.println("fcmpl:");
        System.out.println(resultABL);  // Should print -1
        System.out.println(resultBAL);  // Should print -1
        System.out.println(resultBBL);  // Should print -1
    }

    public static int fcmpg(float a, float b) {
        if (a > b) return 1;
        if (a < b) return -1;
        if (a == b) return 0;

        return 1;  // fcmpg pushes 1 when either a or b is NaN
    }

    public static int fcmpl(float a, float b) {
        if (a > b) return 1;
        if (a < b) return -1;
        if (a == b) return 0;

        return -1;  // fcmpl pushes -1 when either a or b is NaN
    }
}
