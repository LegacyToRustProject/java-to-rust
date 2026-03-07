import java.util.*;
import java.util.stream.*;

public class StreamOps {
    public List<String> filterAndTransform(List<String> items) {
        return items.stream()
            .filter(s -> s.length() > 3)
            .map(String::toUpperCase)
            .sorted()
            .collect(Collectors.toList());
    }

    public int sumOfSquares(List<Integer> numbers) {
        return numbers.stream()
            .map(n -> n * n)
            .reduce(0, Integer::sum);
    }

    public Optional<String> findFirst(List<String> items, String prefix) {
        return items.stream()
            .filter(s -> s.startsWith(prefix))
            .findFirst();
    }

    public Map<Integer, List<String>> groupByLength(List<String> words) {
        return words.stream()
            .collect(Collectors.groupingBy(String::length));
    }
}
