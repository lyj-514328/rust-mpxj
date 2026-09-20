// 对拍专用：与 examples/json_snapshot.rs 读取同一批 ProjectFile 字段。
// 不修改 Java 上游源码；运行时使用子模块编译出的类与其 Maven 依赖。

import java.nio.file.Path;
import java.time.LocalDateTime;
import java.time.format.DateTimeFormatter;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collection;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.TreeMap;

import com.fasterxml.jackson.core.json.JsonWriteFeature;
import com.fasterxml.jackson.databind.ObjectMapper;
import org.mpxj.Duration;
import org.mpxj.FieldContainer;
import org.mpxj.FieldType;
import org.mpxj.LocalDateTimeRange;
import org.mpxj.MpxjEnum;
import org.mpxj.Notes;
import org.mpxj.Priority;
import org.mpxj.ProjectField;
import org.mpxj.ProjectFile;
import org.mpxj.Rate;
import org.mpxj.Relation;
import org.mpxj.Resource;
import org.mpxj.ResourceField;
import org.mpxj.ResourceAssignment;
import org.mpxj.Step;
import org.mpxj.AssignmentField;
import org.mpxj.Task;
import org.mpxj.TaskField;
import org.mpxj.UserDefinedField;
import org.mpxj.TimephasedCost;
import org.mpxj.TimephasedWork;
import org.mpxj.WorkContour;
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

   // 字段快照以枚举名为键；未知值不能悄悄变成字符串或 null。
   private static Object fieldValue(Object value)
   {
      if (value == null || value instanceof String || value instanceof Number || value instanceof Boolean)
      {
         return value;
      }
      if (value instanceof byte[])
      {
         return java.util.HexFormat.of().formatHex((byte[]) value);
      }
      if (value instanceof Duration)
      {
         return duration((Duration) value);
      }
      if (value instanceof Rate)
      {
         Rate rate = (Rate) value;
         return fields("amount", rate.getAmount(), "unit", rate.getUnits().getValue());
      }
      if (value instanceof WorkContour)
      {
         WorkContour contour = (WorkContour) value;
         return fields("unique_id", contour.getUniqueID(), "name", contour.getName());
      }
      if (value instanceof Relation)
      {
         Relation relation = (Relation) value;
         return fields("unique_id", relation.getUniqueID(),
            "predecessor_unique_id", relation.getPredecessorTask().getUniqueID(),
            "successor_unique_id", relation.getSuccessorTask().getUniqueID(),
            "type", relation.getType().getValue(), "lag", duration(relation.getLag()));
      }
      if (value instanceof Resource)
      {
         return fields("unique_id", ((Resource) value).getUniqueID());
      }
      if (value instanceof Notes)
      {
         return value.toString();
      }
      if (value instanceof LocalDateTimeRange)
      {
         LocalDateTimeRange range = (LocalDateTimeRange) value;
         return fields("start", date(range.getStart()), "end", date(range.getEnd()));
      }
      if (value instanceof Step)
      {
         Step step = (Step) value;
         return fields("unique_id", step.getUniqueID(), "name", step.getName(),
            "percent_complete", step.getPercentComplete(), "sequence_number", step.getSequenceNumber(),
            "weight", step.getWeight(), "description", step.getDescription());
      }
      if (value instanceof TimephasedWork)
      {
         TimephasedWork item = (TimephasedWork) value;
         return fields("start", date(item.getStart()), "finish", date(item.getFinish()),
            "total_amount", duration(item.getTotalAmount()), "amount_per_hour", duration(item.getAmountPerHour()));
      }
      if (value instanceof TimephasedCost)
      {
         TimephasedCost item = (TimephasedCost) value;
         return fields("start", date(item.getStart()), "finish", date(item.getFinish()),
            "total_amount", item.getTotalAmount(), "amount_per_hour", item.getAmountPerHour());
      }
      if (value instanceof List<?>)
      {
         List<Object> items = new ArrayList<>();
         for (Object item : (List<?>) value) { items.add(fieldValue(item)); }
         return items;
      }
      if (value instanceof Map<?, ?>)
      {
         Map<String, Object> items = new TreeMap<>();
         for (Map.Entry<?, ?> item : ((Map<?, ?>) value).entrySet())
         {
            if (!(item.getKey() instanceof String)) { throw new IllegalArgumentException("字段 Map 的键不是字符串"); }
            items.put((String) item.getKey(), fieldValue(item.getValue()));
         }
         List<Object> result = new ArrayList<>();
         for (Map.Entry<String, Object> item : items.entrySet())
         {
            result.add(fields("name", item.getKey(), "value", item.getValue()));
         }
         return result;
      }
      if (value instanceof LocalDateTime)
      {
         return date((LocalDateTime) value);
      }
      if (value instanceof java.time.LocalDate || value instanceof java.time.LocalTime || value instanceof java.util.UUID || value instanceof Character)
      {
         if (value instanceof java.time.LocalTime)
         {
            return ((java.time.LocalTime) value).format(DateTimeFormatter.ofPattern("HH:mm:ss.SSSSSSSSS"));
         }
         return value.toString();
      }
      if (value instanceof MpxjEnum)
      {
         return ((MpxjEnum) value).getValue();
      }
      if (value instanceof Priority)
      {
         return ((Priority) value).getValue();
      }
      if (value instanceof Enum<?>)
      {
         if (value instanceof java.time.DayOfWeek)
         {
            return ((java.time.DayOfWeek) value).getValue();
         }
         if (value instanceof org.mpxj.CodePage)
         {
            return ((Enum<?>) value).name();
         }
         if (value instanceof org.mpxj.RateSource)
         {
            return ((Enum<?>) value).name().toLowerCase(Locale.ROOT);
         }
         return ((Enum<?>) value).ordinal();
      }
      throw new IllegalArgumentException("未支持的字段值类型: " + value.getClass().getName());
   }

   private static List<Object> fieldValues(FieldContainer container, FieldType[] types)
   {
      Map<String, Object> values = new TreeMap<>();
      for (FieldType type : types)
      {
         if (excludedField(fieldName(type))) { continue; }
         if (type == ProjectField.CURRENT_DATE && container.getCachedValue(type) == null)
         {
            values.put(fieldName(type), "<dynamic>");
            continue;
         }
         Object value = container.get(type);
         if (value != null)
         {
            try
            {
               values.put(fieldName(type), fieldValue(value));
            }
            catch (RuntimeException ex)
            {
               throw new IllegalStateException(fieldName(type) + ": " + ex.getMessage(), ex);
            }
         }
      }
      List<Object> result = new ArrayList<>();
      for (Map.Entry<String, Object> item : values.entrySet())
      {
         result.add(fields("name", item.getKey(), "value", item.getValue()));
      }
      return result;
   }

   private static boolean excludedField(String name)
   {
      switch (name)
      {
         case "PROJECT_CODE_VALUES":
         case "RESOURCE_CODE_VALUES":
         case "ROLE_CODE_VALUES":
         case "RESOURCE_ASSIGNMENT_CODE_VALUES":
         case "ACTIVITY_CODE_VALUES":
         case "EXPENSE_ITEMS":
            return true;
         default:
            return false;
      }
   }

   private static List<String> excludedNames(FieldType[] types)
   {
      List<String> result = new ArrayList<>();
      for (FieldType type : types) { if (excludedField(type.name())) { result.add(type.name()); } }
      result.sort(String::compareTo);
      return result;
   }

   private static List<String> fieldNames(FieldType[] types)
   {
      List<String> names = new ArrayList<>();
      for (FieldType type : types)
      {
         names.add(fieldName(type));
      }
      names.sort(String::compareTo);
      return names;
   }

   private static String fieldName(FieldType type)
   {
      return type instanceof UserDefinedField ? "CUSTOM:" + ((UserDefinedField) type).getUniqueID() : type.name();
   }

   private static FieldType[] withCustomFields(FieldType[] builtIn, Collection<UserDefinedField> custom)
   {
      List<FieldType> all = new ArrayList<>(Arrays.asList(builtIn));
      all.addAll(custom);
      return all.toArray(new FieldType[0]);
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

   private static Object snapshot(ProjectFile file, boolean allFields)
   {
      var p = file.getProjectProperties();
      FieldType[] projectFields = withCustomFields(ProjectField.values(), file.getUserDefinedFields().getProjectFields());
      FieldType[] taskFields = withCustomFields(TaskField.values(), file.getUserDefinedFields().getTaskFields());
      FieldType[] resourceFields = withCustomFields(ResourceField.values(), file.getUserDefinedFields().getResourceFields());
      FieldType[] assignmentFields = withCustomFields(AssignmentField.values(), file.getUserDefinedFields().getAssignmentFields());
      List<Object> tasks = new ArrayList<>();
      for (Task t : file.getTasks())
      {
         Map<String, Object> entry = fields(
            "id", t.getID(), "unique_id", t.getUniqueID(),
            "parent_unique_id", t.getParentTaskUniqueID(),
            "name", t.getName(), "wbs", t.getWBS(),
            "outline_level", t.getOutlineLevel(),
            "start", date(t.getStart()), "finish", date(t.getFinish()),
            "duration", duration(t.getDuration()), "work", duration(t.getWork()),
            "cost", t.getCost(), "percentage_complete", t.getPercentageComplete(),
            "summary", t.getSummary(), "milestone", t.getMilestone(),
            "calendar_unique_id", t.getCalendarUniqueID());
         if (allFields) { entry.put("field_values", fieldValues(t, taskFields)); }
         tasks.add(entry);
      }
      List<Object> resources = new ArrayList<>();
      for (Resource r : file.getResources())
      {
         Map<String, Object> entry = fields(
            "id", r.getID(), "unique_id", r.getUniqueID(), "name", r.getName(),
            "calendar_unique_id", r.getCalendarUniqueID(),
            "work", duration(r.getWork()), "cost", r.getCost());
         if (allFields) { entry.put("field_values", fieldValues(r, resourceFields)); }
         resources.add(entry);
      }
      List<Object> assignments = new ArrayList<>();
      for (ResourceAssignment a : file.getResourceAssignments())
      {
         Map<String, Object> entry = fields(
            "unique_id", a.getUniqueID(),
            "task_unique_id", a.getTaskUniqueID(), "resource_unique_id", a.getResourceUniqueID(),
            "start", date(a.getStart()), "finish", date(a.getFinish()),
            "work", duration(a.getWork()), "cost", a.getCost());
         if (allFields) { entry.put("field_values", fieldValues(a, assignmentFields)); }
         assignments.add(entry);
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
      Map<String, Object> properties = fields(
            "unique_id", p.getUniqueID(), "name", p.getName(),
            "author", p.getAuthor(), "company", p.getCompany(),
            "start", date(p.getStartDate()), "finish", date(p.getFinishDate()),
            "default_calendar_unique_id", p.getDefaultCalendarUniqueID());
      if (allFields) { properties.put("field_values", fieldValues(p, projectFields)); }
      Map<String, Object> result = fields(
         "properties", properties, "tasks", tasks, "resources", resources, "assignments", assignments,
         "relations", relations, "calendars", calendars);
      if (allFields)
      {
         result.put("field_catalog", fields(
            "properties", fieldNames(projectFields),
            "tasks", fieldNames(taskFields),
            "resources", fieldNames(resourceFields),
            "assignments", fieldNames(assignmentFields)));
         result.put("field_exclusions", fields(
            "properties", excludedNames(ProjectField.values()),
            "tasks", excludedNames(TaskField.values()),
            "resources", excludedNames(ResourceField.values()),
            "assignments", excludedNames(AssignmentField.values())));
      }
      return result;
   }

   public static void main(String[] args) throws Exception
   {
      if (args.length < 1 || args.length > 2 || (args.length == 2 && !"--all-fields".equals(args[1])))
      {
         System.err.println("用法: JsonSnapshot <file.mpx|file.mpp> [--all-fields]");
         System.exit(2);
      }
      // 上游及其依赖可能向 stdout 打印诊断，快照通道只能包含一份 JSON。
      var output = System.out;
      Object value;
      try
      {
         System.setOut(System.err);
         value = snapshot(read(Path.of(args[0])), args.length == 2);
      }
      finally
      {
         System.setOut(output);
      }
      // 在使用 GBK 等系统编码时转义非 ASCII 字符，避免 stdout 将项目名称替换成 '?'。
      ObjectMapper mapper = new ObjectMapper();
      mapper.getFactory().configure(JsonWriteFeature.ESCAPE_NON_ASCII.mappedFeature(), true);
      output.println(mapper.writeValueAsString(value));
   }
}
