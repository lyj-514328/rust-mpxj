// Purpose-built for comparison: reads the same set of ProjectFile fields as examples/json_snapshot.rs.
// Does not modify the Java upstream source; at runtime it uses the classes compiled from the submodule and their Maven dependencies.

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.LocalDateTime;
import java.time.LocalDate;
import java.time.LocalTime;
import java.time.format.DateTimeFormatter;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collection;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.TreeMap;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.TimeUnit;

import com.fasterxml.jackson.core.json.JsonWriteFeature;
import com.fasterxml.jackson.databind.ObjectMapper;
import org.mpxj.Duration;
import org.mpxj.FieldContainer;
import org.mpxj.FieldType;
import org.mpxj.LocalDateTimeRange;
import org.mpxj.MpxjEnum;
import org.mpxj.Notes;
import org.mpxj.Priority;
import org.mpxj.ProjectCalendar;
import org.mpxj.ProjectCalendarWeek;
import org.mpxj.ProjectCalendarException;
import org.mpxj.ProjectCalendarDays;
import org.mpxj.ProjectCalendarHours;
import org.mpxj.LocalTimeRange;
import org.mpxj.RecurringData;
import org.mpxj.CostRateTable;
import org.mpxj.CostRateTableEntry;
import org.mpxj.Availability;
import org.mpxj.View;
import org.mpxj.Table;
import org.mpxj.Column;
import org.mpxj.Filter;
import org.mpxj.GenericCriteria;
import org.mpxj.GenericCriteriaPrompt;
import org.mpxj.Group;
import org.mpxj.GroupClause;
import org.mpxj.DataLink;
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

   // Write the field even when its value is null, so a missing field cannot mask a difference.
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

   // Preserve array order (Sunday before Monday) and distinguish a missing entry from an empty work period.
   private static Object calendarDays(ProjectCalendarDays days)
   {
      List<Object> result = new ArrayList<>();
      var types = days.getCalendarDayTypes();
      var hours = days.getCalendarHours();
      for (int day = 0; day < 7; day++)
      {
         List<Object> ranges = null;
         if (hours[day] != null)
         {
            ranges = new ArrayList<>();
            for (LocalTimeRange range : hours[day])
            {
               ranges.add(fields("start", time(range.getStart()), "end", time(range.getEnd())));
            }
         }
         result.add(fields("type", types[day] == null ? null : types[day].getValue(), "hours", ranges));
      }
      return result;
   }

   private static String time(LocalTime value)
   {
      return value == null ? null : value.format(DateTimeFormatter.ofPattern("HH:mm:ss.SSSSSSSSS"));
   }

   private static String day(LocalDate value)
   {
      return value == null ? null : value.toString();
   }

   private static Object recurrence(RecurringData value)
   {
      if (value == null) { return null; }
      List<Object> weeklyDays = new ArrayList<>();
      for (java.time.DayOfWeek weekday : java.time.DayOfWeek.values())
      {
         weeklyDays.add(value.getWeeklyDay(weekday));
      }
      return fields("start", day(value.getStartDate()), "finish", day(value.getFinishDate()),
         "occurrences", value.getOccurrences(), "type", value.getRecurrenceType() == null ? null : value.getRecurrenceType().getValue(),
         "use_end_date", value.getUseEndDate(), "working_days_only", value.getWorkingDaysOnly(),
         "weekly_days", weeklyDays, "relative", value.getRelative(), "frequency", value.getFrequency(),
         "day_of_week", value.getDayOfWeek() == null ? null : value.getDayOfWeek().getValue(),
         "day_number", value.getDayNumber(), "month_number", value.getMonthNumber());
   }

   private static Object calendarException(ProjectCalendarException value)
   {
      List<Object> hours = new ArrayList<>();
      for (LocalTimeRange range : value)
      {
         hours.add(fields("start", time(range.getStart()), "end", time(range.getEnd())));
      }
      return fields("name", value.getName(), "from", day(value.getFromDate()), "to", day(value.getToDate()),
         "recurring", recurrence(value.getRecurring()), "working", value.getWorking(), "hours", hours);
   }

   private static Object calendarDetails(ProjectCalendar calendar)
   {
      List<Object> weeks = new ArrayList<>();
      for (ProjectCalendarWeek week : calendar.getWorkWeeks())
      {
         var range = week.getDateRange();
         weeks.add(fields("name", week.getName(), "range", range == null ? null :
            fields("start", day(range.getStart()), "end", day(range.getEnd())), "days", calendarDays(week)));
      }
      List<Object> exceptions = new ArrayList<>();
      for (ProjectCalendarException exception : calendar.getCalendarExceptions())
      {
         exceptions.add(calendarException(exception));
      }
      return fields("default", calendar.getDefault(), "guid", calendar.getGUID() == null ? null : calendar.getGUID().toString(),
         "calendar_minutes_per_day", calendar.getCalendarMinutesPerDay(),
         "calendar_minutes_per_week", calendar.getCalendarMinutesPerWeek(),
         "calendar_minutes_per_month", calendar.getCalendarMinutesPerMonth(),
         "calendar_minutes_per_year", calendar.getCalendarMinutesPerYear(),
         "days", calendarDays(calendar), "work_weeks", weeks, "exceptions", exceptions);
   }

   private static Object resourceDetails(Resource resource)
   {
      List<Object> availability = new ArrayList<>();
      for (Availability item : resource.getAvailability())
      {
         availability.add(fields("start", date(item.getRange().getStart()),
            "end", date(item.getRange().getEnd()), "units", item.getUnits()));
      }
      List<Object> tables = new ArrayList<>();
      for (int index = 0; index < 5; index++)
      {
         CostRateTable table = resource.getCostRateTable(index);
         if (table == null) { tables.add(null); continue; }
         List<Object> entries = new ArrayList<>();
         for (CostRateTableEntry entry : table)
         {
            List<Object> rates = new ArrayList<>();
            for (int rateIndex = 0; rateIndex < 5; rateIndex++)
            {
               rates.add(fieldValue(entry.getRate(rateIndex)));
            }
            entries.add(fields("start", date(entry.getStartDate()), "end", date(entry.getEndDate()),
               "rates", rates, "cost_per_use", entry.getCostPerUse()));
         }
         tables.add(entries);
      }
      return fields("availability", availability, "cost_rate_tables", tables);
   }

   private static Object table(Table value)
   {
      if (value == null) { return null; }
      List<Object> columns = new ArrayList<>();
      for (Column column : value.getColumns())
      {
         columns.add(fields("field", column.getFieldType() == null ? null : fieldName(column.getFieldType()),
            "width", column.getWidth(), "title", column.getTitle(),
            "align_title", column.getAlignTitle(), "align_data", column.getAlignData()));
      }
      return fields("id", value.getID(), "name", value.getName(),
         "resource", value.getResourceFlag(), "columns", columns);
   }

   private static Object view(View value)
   {
      return fields("id", value.getID(), "name", value.getName(),
         "type", value.getType() == null ? null : value.getType().getValue(),
         "table_name", value.getTableName(),
         "table_id", value.getTable() == null ? null : value.getTable().getID());
   }

   private static Object criteriaValue(Object value)
   {
      if (value instanceof FieldType) { return fields("field", fieldName((FieldType) value)); }
      if (value instanceof GenericCriteriaPrompt)
      {
         GenericCriteriaPrompt prompt = (GenericCriteriaPrompt) value;
         return fields("prompt", prompt.getPrompt(), "type", prompt.getType().getValue());
      }
      return fieldValue(value);
   }

   private static Object criteria(GenericCriteria value)
   {
      if (value == null) { return null; }
      List<Object> children = new ArrayList<>();
      for (GenericCriteria child : value.getCriteriaList()) { children.add(criteria(child)); }
      return fields("field", value.getLeftValue() == null ? null : fieldName(value.getLeftValue()),
         "operator", value.getOperator() == null ? null : value.getOperator().getValue(),
         "values", Arrays.asList(criteriaValue(value.getValue(0)), criteriaValue(value.getValue(1))),
         "children", children);
   }

   private static Object filter(Filter value)
   {
      List<Object> prompts = null;
      if (value.getPrompts() != null)
      {
         prompts = new ArrayList<>();
         for (GenericCriteriaPrompt prompt : value.getPrompts())
         {
            prompts.add(fields("prompt", prompt.getPrompt(), "type", prompt.getType().getValue()));
         }
      }
      return fields("id", value.getID(), "name", value.getName(),
         "show_related_summary_rows", value.getShowRelatedSummaryRows(),
         "task", value.isTaskFilter(), "resource", value.isResourceFilter(),
         "criteria", criteria(value.getCriteria()), "prompts", prompts);
   }

   private static Object group(Group value)
   {
      List<Object> clauses = new ArrayList<>();
      for (GroupClause clause : value.getGroupClauses())
      {
         clauses.add(fields("ascending", clause.getAscending(),
            "field", clause.getField() == null ? null : fieldName(clause.getField()),
            "group_interval", criteriaValue(clause.getGroupInterval()),
            "group_on", clause.getGroupOn(), "start_at", criteriaValue(clause.getStartAt())));
      }
      return fields("id", value.getID(), "name", value.getName(),
         "show_summary_tasks", value.getShowSummaryTasks(), "clauses", clauses);
   }

   // The field snapshot is keyed by enum name; an unknown value must not silently become a string or null.
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
            if (!(item.getKey() instanceof String)) { throw new IllegalArgumentException("A key in the fields Map is not a string"); }
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
      throw new IllegalArgumentException("Unsupported field value type: " + value.getClass().getName());
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
      throw new IllegalArgumentException("Only .mpp and .mpx are supported");
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
         if (allFields) { entry.put("field_values", fieldValues(r, resourceFields)); entry.put("details", resourceDetails(r)); }
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
         Map<String, Object> calendar = fields(
            "unique_id", c.getUniqueID(), "name", c.getName(),
            "parent_unique_id", c.getParent() == null ? null : c.getParent().getUniqueID(),
            "project_unique_id", c.getProjectUniqueID(), "type", c.getType().ordinal());
         if (allFields) { calendar.put("details", calendarDetails(c)); }
         calendars.add(calendar);
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
         List<Object> tables = new ArrayList<>();
         for (Table table : file.getTables()) { tables.add(table(table)); }
         result.put("tables", tables);
         List<Object> views = new ArrayList<>();
         for (View view : file.getViews()) { views.add(view(view)); }
         result.put("views", views);
         var viewState = file.getViews().getViewState();
         result.put("view_state", viewState == null ? null : fields("view_name", viewState.getViewName(),
            "unique_ids", viewState.getUniqueIdList(),
            "filter_id", viewState.getFilter() == null ? null : viewState.getFilter().getID()));
         List<Object> groups = new ArrayList<>();
         for (Group group : file.getGroups()) { groups.add(group(group)); }
         result.put("groups", groups);
         List<Object> taskFilters = new ArrayList<>();
         for (Filter filter : file.getFilters().getTaskFilters()) { taskFilters.add(filter(filter)); }
         result.put("task_filters", taskFilters);
         List<Object> resourceFilters = new ArrayList<>();
         for (Filter filter : file.getFilters().getResourceFilters()) { resourceFilters.add(filter(filter)); }
         result.put("resource_filters", resourceFilters);
         List<Object> definedFields = new ArrayList<>();
         for (UserDefinedField field : file.getUserDefinedFields())
         {
            definedFields.add(fields("unique_id", field.getUniqueID(), "internal_name", field.name(),
               "name", field.getName(), "summary_task_only", field.getSummaryTaskOnly(),
               "field_type_class", field.getFieldTypeClass() == null ? null : field.getFieldTypeClass().ordinal(),
               "data_type", field.getDataType() == null ? null : field.getDataType().getValue()));
         }
         result.put("user_defined_fields", definedFields);
         List<Object> contours = new ArrayList<>();
         for (WorkContour contour : file.getWorkContours())
         {
            // MPXJ's ten built-in contours are static WorkContour instances and never appear in the container collection;
            // only serialize the extended contours actually registered by the file.
            if (contour.getUniqueID() != null && contour.getUniqueID() >= 0 && contour.getUniqueID() <= 9) { continue; }
            contours.add(fields("unique_id", contour.getUniqueID(), "name", contour.getName(),
               "curve", contour.getCurveValues()));
         }
         result.put("work_contours", contours);
         List<Object> dataLinks = new ArrayList<>();
         for (DataLink link : file.getDataLinks())
         {
            dataLinks.add(fields("id", link.getID(),
               "source_field", link.getSourceField() == null ? null : fieldName(link.getSourceField()),
               "source_unique_id", link.getSourceUniqueID(),
               "target_field", link.getTargetField() == null ? null : fieldName(link.getTargetField()),
               "target_unique_id", link.getTargetUniqueID()));
         }
         result.put("data_links", dataLinks);
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

   // Server mode: each stdin line is "<stem>\t<sample-path>"; the result is written to <output-dir>/<stem>.json.
   // Each file looks like {"path": "...", "result": {...}}; on read failure "result" is replaced by "error".
   // The whole process redirects System.out to stderr so upstream diagnostics cannot pollute the protocol output.
   private static void runServer(String[] args) throws Exception
   {
      Path outputDir = null;
      boolean allFields = false;
      int threads = Runtime.getRuntime().availableProcessors();
      for (int index = 0; index < args.length; index++)
      {
         switch (args[index])
         {
            case "--all-fields":
               allFields = true;
               break;
            case "--threads":
               if (++index >= args.length) { throw new IllegalArgumentException("--threads is missing its argument"); }
               threads = Integer.parseInt(args[index]);
               break;
            default:
               if (outputDir != null) { throw new IllegalArgumentException("Unexpected argument: " + args[index]); }
               outputDir = Path.of(args[index]);
         }
      }
      if (outputDir == null) { throw new IllegalArgumentException("--server requires an output directory"); }
      if (threads < 1) { throw new IllegalArgumentException("--threads must be a positive integer"); }
      Files.createDirectories(outputDir);

      PrintStream console = System.out;
      // Escape non-ASCII characters when using a system encoding such as GBK, so project names are not replaced with '?'.
      final ObjectMapper mapper = new ObjectMapper();
      mapper.getFactory().configure(JsonWriteFeature.ESCAPE_NON_ASCII.mappedFeature(), true);
      final Path targetDir = outputDir;
      System.setOut(System.err);

      ExecutorService pool = Executors.newFixedThreadPool(threads);
      int submitted = 0;
      try (BufferedReader reader = new BufferedReader(new InputStreamReader(System.in, StandardCharsets.UTF_8)))
      {
         String line;
         while ((line = reader.readLine()) != null)
         {
            String entry = line.strip();
            if (entry.isEmpty() || entry.startsWith("#")) { continue; }
            int tab = entry.indexOf('\t');
            if (tab < 0)
            {
               console.println("Skipping input line without a tab: " + entry);
               continue;
            }
            final String stem = entry.substring(0, tab).strip();
            final String sample = entry.substring(tab + 1).strip();
            if (!stem.matches("[A-Za-z0-9._-]+"))
            {
               console.println("Skipping invalid output name: " + stem);
               continue;
            }
            final boolean fields = allFields;
            pool.submit(() -> writeSnapshot(targetDir, stem, sample, fields, mapper));
            submitted++;
         }
      }
      finally
      {
         pool.shutdown();
         pool.awaitTermination(1, TimeUnit.DAYS);
      }
      console.println("server finished: " + submitted + " samples, " + threads + " threads");
   }

   // Single sample: read and serialize, then write in one shot so a half-written JSON is never read.
   private static void writeSnapshot(Path outputDir, String stem, String sample, boolean allFields, ObjectMapper mapper)
   {
      Map<String, Object> wrapper = new LinkedHashMap<>();
      wrapper.put("path", sample);
      try
      {
         wrapper.put("result", snapshot(read(Path.of(sample)), allFields));
      }
      catch (Exception ex)
      {
         wrapper.put("error", ex.getClass().getSimpleName() + ": " + String.valueOf(ex.getMessage()));
      }
      Path target = outputDir.resolve(stem + ".json");
      try
      {
         Files.writeString(target, mapper.writeValueAsString(wrapper), StandardCharsets.UTF_8);
      }
      catch (Exception ex)
      {
         System.err.println("Failed to write snapshot " + target + ": " + ex);
      }
   }

   public static void main(String[] args) throws Exception
   {
      if (args.length > 0 && "--server".equals(args[0]))
      {
         try
         {
            runServer(Arrays.copyOfRange(args, 1, args.length));
         }
         catch (Exception ex)
         {
            System.err.println("server failed to start: " + ex.getMessage());
            System.exit(2);
         }
         return;
      }
      if (args.length < 1 || args.length > 2 || (args.length == 2 && !"--all-fields".equals(args[1])))
      {
         System.err.println("Usage: JsonSnapshot <file.mpx|file.mpp> [--all-fields]");
         System.err.println("      JsonSnapshot --server <output-dir> [--all-fields] [--threads N]");
         System.exit(2);
      }
      // Upstream and its dependencies may print diagnostics to stdout; the snapshot channel may contain only one JSON document.
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
      // Escape non-ASCII characters when using a system encoding such as GBK, so stdout does not replace project names with '?'.
      ObjectMapper mapper = new ObjectMapper();
      mapper.getFactory().configure(JsonWriteFeature.ESCAPE_NON_ASCII.mappedFeature(), true);
      output.println(mapper.writeValueAsString(value));
   }
}
