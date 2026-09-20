// 对拍专用：与 examples/json_snapshot.rs 读取同一批 ProjectFile 字段。
// 不修改 Java 上游源码；运行时使用子模块编译出的类与其 Maven 依赖。

import java.nio.file.Path;
import java.time.LocalDateTime;
import java.time.format.DateTimeFormatter;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;

import com.fasterxml.jackson.databind.ObjectMapper;
import org.mpxj.Duration;
import org.mpxj.ProjectFile;
import org.mpxj.Relation;
import org.mpxj.Resource;
import org.mpxj.ResourceAssignment;
import org.mpxj.Task;
import org.mpxj.mpp.MPPReader;
import org.mpxj.mpx.MPXReader;

public final class JsonSnapshot
{
   private static final DateTimeFormatter DATE = DateTimeFormatter.ofPattern("yyyy-MM-dd'T'HH:mm:ss.SSSSSSSSS");

   // 即使字段值为 null 也写入，不能让缺失字段掩盖差异。
   private static Map<String, Object> fields(Object... values)
   {
      Map<String, Object> result = new LinkedHashMap<>();
      for (int i = 0; i < values.length; i += 2)
      {
         result.put((String) values[i], values[i + 1]);
      }
      return result;
   }

   private static String date(LocalDateTime value)
   {
      return value == null ? null : DATE.format(value);
   }

   private static Map<String, Object> duration(Duration value)
   {
      return value == null ? null : fields("value", value.getDuration(), "unit", value.getUnits().getValue());
   }

   private static ProjectFile read(Path path) throws Exception
   {
      String name = path.getFileName().toString().toLowerCase(Locale.ROOT);
      if (name.endsWith(".mpp"))
      {
         return new MPPReader().read(path.toFile());
      }
      if (name.endsWith(".mpx"))
      {
         MPXReader reader = new MPXReader();
         String stem = name.substring(0, name.length() - 4);
         int dot = stem.lastIndexOf('.');
         String language = dot < 0 ? "en" : stem.substring(dot + 1);
         switch (language)
         {
            case "de": case "fr": case "it": case "pt": case "sv": case "zh": case "es": case "ru":
               reader.setLocale(Locale.forLanguageTag(language));
               break;
            default:
               reader.setLocale(Locale.ENGLISH);
         }
         return reader.read(path.toFile());
      }
      throw new IllegalArgumentException("仅支持 .mpp 和 .mpx");
   }

   private static Object snapshot(ProjectFile file)
   {
      var p = file.getProjectProperties();
      List<Object> tasks = new ArrayList<>();
      for (Task t : file.getTasks())
      {
         tasks.add(fields(
            "id", t.getID(), "unique_id", t.getUniqueID(),
            "parent_unique_id", t.getParentTaskUniqueID(),
            "name", t.getName(), "wbs", t.getWBS(),
            "outline_level", t.getOutlineLevel(),
            "start", date(t.getStart()), "finish", date(t.getFinish()),
            "duration", duration(t.getDuration()), "work", duration(t.getWork()),
            "cost", t.getCost(), "percentage_complete", t.getPercentageComplete(),
            "summary", t.getSummary(), "milestone", t.getMilestone(),
            "calendar_unique_id", t.getCalendarUniqueID()));
      }
      List<Object> resources = new ArrayList<>();
      for (Resource r : file.getResources())
      {
         resources.add(fields(
            "id", r.getID(), "unique_id", r.getUniqueID(), "name", r.getName(),
            "calendar_unique_id", r.getCalendarUniqueID(),
            "work", duration(r.getWork()), "cost", r.getCost()));
      }
      List<Object> assignments = new ArrayList<>();
      for (ResourceAssignment a : file.getResourceAssignments())
      {
         assignments.add(fields(
            "unique_id", a.getUniqueID(),
            "task_unique_id", a.getTaskUniqueID(), "resource_unique_id", a.getResourceUniqueID(),
            "start", date(a.getStart()), "finish", date(a.getFinish()),
            "work", duration(a.getWork()), "cost", a.getCost()));
      }
      List<Object> relations = new ArrayList<>();
      for (Relation r : file.getRelations())
      {
         relations.add(fields(
            "unique_id", r.getUniqueID(),
            "predecessor_unique_id", r.getPredecessorTask().getUniqueID(),
            "successor_unique_id", r.getSuccessorTask().getUniqueID(),
            "type", r.getType().getValue(), "lag", duration(r.getLag())));
      }
      List<Object> calendars = new ArrayList<>();
      for (var c : file.getCalendars())
      {
         calendars.add(fields(
            "unique_id", c.getUniqueID(), "name", c.getName(),
            "parent_unique_id", c.getParent() == null ? null : c.getParent().getUniqueID(),
            "project_unique_id", c.getProjectUniqueID(), "type", c.getType().ordinal()));
      }
      return fields(
         "properties", fields(
            "unique_id", p.getUniqueID(), "name", p.getName(),
            "author", p.getAuthor(), "company", p.getCompany(),
            "start", date(p.getStartDate()), "finish", date(p.getFinishDate()),
            "default_calendar_unique_id", p.getDefaultCalendarUniqueID()),
         "tasks", tasks, "resources", resources, "assignments", assignments,
         "relations", relations, "calendars", calendars);
   }

   public static void main(String[] args) throws Exception
   {
      if (args.length != 1)
      {
         System.err.println("用法: JsonSnapshot <file.mpx|file.mpp>");
         System.exit(2);
      }
      // 上游及其依赖可能向 stdout 打印诊断，快照通道只能包含一份 JSON。
      var output = System.out;
      Object value;
      try
      {
         System.setOut(System.err);
         value = snapshot(read(Path.of(args[0])));
      }
      finally
      {
         System.setOut(output);
      }
      output.println(new ObjectMapper().writeValueAsString(value));
   }
}
