public class InClassCall {
    public static void main(String[] args) {
        var i = getArgsLen(args);
        printIntDoubled(i);
        printArgs(args);
        printArgLen(args);
        printBoolean(false);
        printBoolean(true);
    }

    static void printArgs(String[] args) {
        for (int i = 0; i < args.length; i++)
            System.out.println(args[i]);
    }

    static void printArgLen(String[] args) {
        System.out.println(args.length);
    }

    static void printIntDoubled(Integer i) {
        i = i * 2;
        System.out.println(i.intValue());
    }

    static int getArgsLen(String[] args) {
        return args.length;
    }

    static void printBoolean(boolean b) {
        b = !b;
        System.out.println(b);
    }
}