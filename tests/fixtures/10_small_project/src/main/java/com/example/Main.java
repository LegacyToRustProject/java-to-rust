package com.example;

public class Main {
    public static void main(String[] args) {
        Calculator calc = new Calculator();
        System.out.println("2 + 3 = " + calc.add(2, 3));
        System.out.println("10 - 4 = " + calc.subtract(10, 4));
        System.out.println("6 * 7 = " + calc.multiply(6, 7));

        try {
            System.out.println("10 / 3 = " + calc.divide(10, 3));
            System.out.println("10 / 0 = " + calc.divide(10, 0));
        } catch (ArithmeticException e) {
            System.out.println("Error: " + e.getMessage());
        }
    }
}
