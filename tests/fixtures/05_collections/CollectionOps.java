import java.util.*;

public class CollectionOps {
    public List<String> processItems(List<String> items) {
        List<String> result = new ArrayList<>();
        for (String item : items) {
            result.add(item.toUpperCase());
        }
        return result;
    }

    public Map<String, Integer> wordCount(List<String> words) {
        Map<String, Integer> counts = new HashMap<>();
        for (String word : words) {
            counts.put(word, counts.getOrDefault(word, 0) + 1);
        }
        return counts;
    }

    public Set<String> uniqueItems(List<String> items) {
        return new HashSet<>(items);
    }
}
