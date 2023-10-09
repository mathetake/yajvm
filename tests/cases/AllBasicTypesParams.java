public class AllBasicTypesParams {
    public static void main(String[] args) {
        modifyAndPrint(true, (byte)5, (short)10, 'A', 15, 20L, 25.0f, 30.0);
        ignoreAllParams(true, (byte)5, (short)10, 'A', 15, 20L, 25.0f, 30.0);
    }

    public static void modifyAndPrint(boolean boolValue, byte byteValue, short shortValue,
                                      char charValue, int intValue, long longValue,
                                      float floatValue, double doubleValue) {

        // Modify and print each value
        System.out.println(!boolValue);   // Expected: false

        byteValue += 1;
        System.out.println(byteValue);   // Expected: 6

        shortValue += 2;
        System.out.println(shortValue);  // Expected: 12

        charValue += 1;
        System.out.println(charValue);   // Expected: B

        intValue += 3;
        System.out.println(intValue);    // Expected: 18

        longValue += 4;
        System.out.println(longValue);   // Expected: 24

        floatValue += 5.0f;
        System.out.println(floatValue);  // Expected: 30.0

        doubleValue += 6.0;
        System.out.println(doubleValue); // Expected: 36.0
    }

    public static void ignoreAllParams(boolean boolValue, byte byteValue, short shortValue,
                                       char charValue, int intValue, long longValue,
                                       float floatValue, double doubleValue) {
        // Do nothing
    }
}
