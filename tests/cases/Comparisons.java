public class Comparisons {
    public static void main(String[] args) {
        // Integer Comparisons
        System.out.println("==Integer==");
        System.out.println(compareInts(5, 3));  // Should print "Greater"
        System.out.println(compareInts(3, 5));  // Should print "Smaller"
        System.out.println(compareInts(3, 3));  // Should print "Equal"

        // Long Comparisons
        System.out.println("==Long==");
        System.out.println(compareLongs(5000000000L, 3000000000L));  // Should print "Greater"
        System.out.println(compareLongs(3000000000L, 5000000000L));  // Should print "Smaller"
        System.out.println(compareLongs(3000000000L, 3000000000L));  // Should print "Equal"

        // Float Comparisons
        System.out.println("==Float==");
        System.out.println(compareFloats(5.5f, 3.3f));  // Should print "Greater"
        System.out.println(compareFloats(3.3f, 5.5f));  // Should print "Smaller"
        System.out.println(compareFloats(3.3f, 3.3f));  // Should print "Equal"

        // Double Comparisons
        System.out.println("==Double==");
        System.out.println(compareDoubles(5.5, 3.3));  // Should print "Greater"
        System.out.println(compareDoubles(3.3, 5.5));  // Should print "Smaller"
        System.out.println(compareDoubles(3.3, 3.3));  // Should print "Equal"
    }

    public static String compareInts(int a, int b) {
        if (a > b) {
            return "Greater";
        } else if (a < b) {
            return "Smaller";
        } else {
            return "Equal";
        }
    }

    public static String compareLongs(long a, long b) {
        if (a > b) {
            return "Greater";
        } else if (a < b) {
            return "Smaller";
        } else {
            return "Equal";
        }
    }

    public static String compareFloats(float a, float b) {
        if (a > b) {
            return "Greater";
        } else if (a < b) {
            return "Smaller";
        } else {
            return "Equal";
        }
    }

    public static String compareDoubles(double a, double b) {
        if (a > b) {
            return "Greater";
        } else if (a < b) {
            return "Smaller";
        } else {
            return "Equal";
        }
    }
}
