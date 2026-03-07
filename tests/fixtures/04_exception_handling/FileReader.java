import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Paths;

public class FileReader {
    public String readFile(String path) throws IOException {
        return new String(Files.readAllBytes(Paths.get(path)));
    }

    public String readFileSafe(String path) {
        try {
            return readFile(path);
        } catch (IOException e) {
            return "Error: " + e.getMessage();
        }
    }
}
