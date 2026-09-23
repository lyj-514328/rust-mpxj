//! Corresponds to Java: `MSPDIReader.readResources` and its helpers
//! (`org/mpxj/mspdi/MSPDIReader.java` lines 883-1195).
//!
//! This module holds the `impl ReaderContext` block for the resource section; it is split out of
//! [`crate::mspdi::mspdi_reader`] so that each section can be compared against its Java line range
//! (see the file header of that module).
//!
//! # Differences from Java
//!
//! | Java | Rust | Notes |
//! |---|---|---|
//! | `Project.Resources resources = project.getResources(); if (resources != null) { ... }` | the flattened `Vec` of `schema::project::Project::resources` is iterated unconditionally | The `Resources` wrapper class is flattened away by the schema layer (see its header): an absent wrapper yields an empty list, so "iterate" and Java's null check have the same effect (Java would throw a `NullPointerException` for an absent wrapper). |
//! | `m_projectFile.addResource()` | `self.file.borrow_mut().add_resource()` | `ProjectFile::add_resource` forwards to `ProjectContext::get_resources_mut().add()`, which also wires the resource's self reference (needed by `setID`). |
//! | `mpx.setAccrueAt` / `setType` / `setWorkGroup` / `setBookingType(xml.getXxx())` pass the nullable JAXB enum straight to `set(FieldType, Object)` | `Resource::set_accrue_at` / `set_type` / `set_work_group` / `set_booking_type` are called only when the XML value is present | The Rust setters take the enum itself, not an `Option`. A freshly created `Resource` has none of these fields set (`Resource::new` leaves the field table empty), so skipping an absent element is observationally identical to Java writing `null`; the `Option`-returning getters still report "absent". |
//! | `DatatypeConverter.parseDuration(m_projectFile, null, value)` | [`resources_parse_duration`] | The frozen `parse_duration` takes `&ProjectFile`; the helper opens the file borrow (using `try_borrow`, so it degrades to `None` rather than panicking when the file is already borrowed — impossible on this call path). The conversion is pure, so evaluating it at the setter's position gives Java's result. |
//! | `DatatypeConverter.parseDurationTimeUnits(value, null)` | [`resources_parse_duration_time_units`] | Java's two-argument overload returns the supplied default (`null`) for a null value and reads the parent file's default duration units in its `default:` branch. Neither frozen Rust overload reproduces both halves (`parse_duration_time_units` returns `HOURS` for an absent value, `..._with_default` carries no file), so the helper combines them. |
//! | `DatatypeConverter.parseTimeUnit(value)` never returns `null` | [`resources_time_unit`] | The frozen Rust signature returns `Option`; the helper resolves the impossible `None` to `DAYS`, which is also Java's fallback for an out-of-range value. |
//! | `DatatypeConverter.parseRate(value, units)` reads the parent file's `ProjectProperties` from the `PARENT_FILE` `ThreadLocal` | [`resources_parse_rate`] uses [`DatatypeConverter::parse_rate_with_properties`] with the properties of the file being read | Faithful call-site choice: the frozen [`DatatypeConverter::parse_rate`] carries no properties and would fall back to the library's own `TimeUnitDefaults`, which is not what Java reads here. |
//! | `NumberHelper.getInteger(BigInteger)` / `getInt(BigInteger)` | the schema stores `Option<i64>`, so the value is widened to `f64` first | Reproduces Java's `BigInteger.doubleValue()`; the Rust `NumberHelper` overloads take `Option<f64>`. |
//! | `mpx.setUnitOfMeasure(m_projectFile.getUnitsOfMeasure().getOrCreateByAbbreviation(xml.getMaterialLabel()))` | [`resources_unit_of_measure`] clones the entry out of the container, then [`Resource::set_unit_of_measure`] stores its unique ID | The container lives in the `ProjectContext` and must be borrowed mutably to create a missing entry; cloning ends that borrow before the resource is borrowed. |
//! | `mpx.setNotes(...)` is only called when the value is neither null nor empty | the same guard on the schema's `Option<String>` | Java's `xml.getNotes() != null && !xml.getNotes().isEmpty()`. |
//! | `mpx.setCalendar(calendarMap.get(xml.getCalendarUID()))` | `calendar_map.get(&uid).cloned()`, then [`Resource::set_calendar`] | Java's `BigInteger` key is an `i64` in the schema layer; the calendar is cloned out of the map because the resource is borrowed mutably afterwards. |
//! | `Project.Resources.Resource.Rates` wrapper and the `rates == null` test | the schema keeps a flattened `Vec` plus `rates_present` | The presence flag distinguishes an absent wrapper (`null`) from an empty `<Rates/>`, so only the former writes the five default cost rate tables. |
//! | `new CostRateTableEntry(start, end, costPerUse, defaultStandardRate, defaultOvertimeRate)` with nullable rates | `CostRateTableEntry::new_with_optional_rates` | Java stores `null` in the rate array when a source rate is absent; the Rust entry tracks per-rate presence while retaining zero values for the existing rate accessors. |
//! | `rate.getRatesFrom()` / `rate.getRatesTo()` are dereferenced (`isBefore` / `isAfter` / `getMinute`) and throw `NullPointerException` for an absent element | the clamp and the minute heuristic are skipped for an absent date | Rust-specific branch: library code never panics. Both elements are `required` in the schema, so real files always carry them. |
//! | `Collections.sort(table)` after appending each entry to the `CostRateTable` | the entries are collected per table index, sorted and then handed to [`CostRateTable::from_entries`] | [`CostRateTable`] has no `sort` (it only exposes the collection operations the port needs) and the entry vector holds the same rows; both sorts are stable and compare end dates, so the resulting table is the same. |
//! | `Collections.sort(table)` after appending each `Availability` to the table | the `Availability` values are collected, sorted and then appended through [`Resource::add_availability`] | [`Resource::get_availability`] lends out a read-only `Ref`, so the table is filled after the sort. Java's "append then sort" and this "sort then append" produce the same table. |
//! | `FieldTypeHelper.getInstance(m_projectFile, id)` returns a `FieldType` that is used as a lookup table key and as the target of `mpx.set(...)` | `MppFieldType::to_field_key()` yields the runtime-erased `FieldKey` (custom fields included) | `dyn FieldType` is not object safe; `FieldKey` is this crate's concrete carrier for a Java `FieldType` reference (see [`crate::field_key`]). |
//! | `getOutlineCodeValue(FieldType mpxFieldID, BigInteger valueID)` | `self.get_outline_code_value(mpx_field_id: Option<FieldKey>, value_id: Option<i64>) -> Option<String>` | The method belongs to the custom field section ([`crate::mspdi::reader_custom_fields`]); the parameter types are the crate's mapping of the Java `FieldType` / `BigInteger` pair. The `FieldID` is never `None` at this call site: an absent, unparsable or unknown field ID skips the record (Java would parse a null `FieldID` with `Integer.parseInt` and throw). |
//! | an unparsable `FieldID` throws `NumberFormatException` (`Integer.parseInt`), aborting the read | the record is skipped (`str::parse().ok()`) | Library code never panics; the surrounding document is still read. |
//! | the commented-out Java calls (`//mpx.setBaseCalendar()`, `//mpx.setBaselineCost()`, `//mpx.setBaselineWork()`, `//mpx.setLinkedFields()`, `//mpx.setObjects()`) | reproduced as comments in [`ReaderContext::read_resource`] | They are commented out in Java as well, so nothing is ported for them. |
//!
//! # Skipped Java members + reason
//!
//! None.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use chrono::{Duration as ChronoDuration, Timelike};

use crate::availability::Availability;
use crate::common::boolean_helper::BooleanHelper;
use crate::common::field_type_helper::FieldTypeHelper;
use crate::common::local_date_time_helper::LocalDateTimeHelper;
use crate::common::number_helper::NumberHelper;
use crate::cost_rate_table::CostRateTable;
use crate::cost_rate_table_entry::CostRateTableEntry;
use crate::duration::Duration;
use crate::field_value::FieldValue;
use crate::mspdi::datatype_converter::DatatypeConverter;
use crate::mspdi::mspdi_reader::ReaderContext;
use crate::mspdi::schema::project::Project;
use crate::mspdi::schema::resources::resource::rates::Rate as XmlRate;
use crate::mspdi::schema::resources::resource::Resource as XmlResource;
use crate::project_calendar::ProjectCalendar;
use crate::project_entity_with_id::ProjectEntityWithID;
use crate::project_entity_with_mutable_unique_id::ProjectEntityWithMutableUniqueID;
use crate::project_file::ProjectFile;
use crate::rate::Rate;
use crate::resource::Resource;
use crate::time_unit::TimeUnit;
use crate::unit_of_measure::UnitOfMeasure;

impl ReaderContext {
    /// This method extracts resource data from an MSPDI file.
    ///
    /// Corresponds to Java: `private void readResources(Project project, HashMap<BigInteger, ProjectCalendar> calendarMap)`
    ///
    /// The Rust `calendar_map` is borrowed (not taken by value) because the calendar section fills
    /// it in and this method only reads it, exactly as in Java.
    pub(crate) fn read_resources(
        &mut self,
        project: &Project,
        calendar_map: &HashMap<i64, Rc<RefCell<ProjectCalendar>>>,
    ) {
        // Java:
        // Project.Resources resources = project.getResources();
        // if (resources != null)
        // {
        //    for (Project.Resources.Resource resource : resources.getResource())
        //    {
        //       readResource(resource, calendarMap);
        //    }
        // }
        //
        // The `Resources` wrapper is flattened away by the schema layer (see its file header): an
        // absent wrapper yields an empty list, so the null check has no counterpart here.
        for xml in &project.resources {
            self.read_resource(xml, calendar_map);
        }
    }

    /// This method extracts data for a single resource from an MSPDI file.
    ///
    /// Corresponds to Java: `private void readResource(Project.Resources.Resource xml, HashMap<BigInteger, ProjectCalendar> calendarMap)`
    ///
    /// The setters run in the order of the Java method (lines 904-988). Two kinds of conversion are
    /// hoisted out of that sequence because the Rust borrow model cannot express them inline: the
    /// `xsd:duration` values (a `&ProjectFile` borrow, evaluated at each setter's position) and the
    /// rates (a `ProjectProperties` borrow); neither conversion has a side effect, so the resulting
    /// field values are the same as Java's.
    fn read_resource(
        &mut self,
        xml: &XmlResource,
        calendar_map: &HashMap<i64, Rc<RefCell<ProjectCalendar>>>,
    ) {
        // Java: Resource mpx = m_projectFile.addResource();
        let resource = self.file.borrow_mut().add_resource();

        // Java: mpx.setAccrueAt(xml.getAccrueAt());
        if let Some(accrue_at) = xml.accrue_at {
            resource.borrow_mut().set_accrue_at(accrue_at);
        }

        // Java: mpx.setActiveDirectoryGUID(xml.getActiveDirectoryGUID());
        resource
            .borrow_mut()
            .set_active_directory_guid(xml.active_directory_guid.clone());

        // Java: mpx.setActualCost(DatatypeConverter.parseCurrency(xml.getActualCost()));
        resource
            .borrow_mut()
            .set_actual_cost(DatatypeConverter::parse_currency(xml.actual_cost));

        // Java: mpx.setActualOvertimeCost(DatatypeConverter.parseCurrency(xml.getActualOvertimeCost()));
        resource
            .borrow_mut()
            .set_actual_overtime_cost(DatatypeConverter::parse_currency(xml.actual_overtime_cost));

        // Java: mpx.setActualOvertimeWork(DatatypeConverter.parseDuration(m_projectFile, null, xml.getActualOvertimeWork()));
        resource.borrow_mut().set_actual_overtime_work(resources_parse_duration(
            &self.file,
            None,
            xml.actual_overtime_work.as_deref(),
        ));

        // Java: mpx.setActualOvertimeWorkProtected(DatatypeConverter.parseDuration(m_projectFile, null, xml.getActualOvertimeWorkProtected()));
        resource
            .borrow_mut()
            .set_actual_overtime_work_protected(resources_parse_duration(
                &self.file,
                None,
                xml.actual_overtime_work_protected.as_deref(),
            ));

        // Java: mpx.setActualWorkProtected(DatatypeConverter.parseDuration(m_projectFile, null, xml.getActualWorkProtected()));
        resource
            .borrow_mut()
            .set_actual_work_protected(resources_parse_duration(
                &self.file,
                None,
                xml.actual_work_protected.as_deref(),
            ));

        // Java: mpx.setACWP(DatatypeConverter.parseCurrency(xml.getACWP()));
        resource
            .borrow_mut()
            .set_acwp(DatatypeConverter::parse_currency(xml.acwp));

        // Java: mpx.setBCWS(DatatypeConverter.parseCurrency(xml.getBCWS()));
        resource
            .borrow_mut()
            .set_bcws(DatatypeConverter::parse_currency(xml.bcws));

        // Java: mpx.setBCWP(DatatypeConverter.parseCurrency(xml.getBCWP()));
        resource
            .borrow_mut()
            .set_bcwp(DatatypeConverter::parse_currency(xml.bcwp));

        // Java: mpx.setBookingType(xml.getBookingType());
        if let Some(booking_type) = xml.booking_type {
            resource.borrow_mut().set_booking_type(booking_type);
        }

        // Java: mpx.setBaseCalendar(); (commented out)
        // Java: mpx.setBaselineCost(); (commented out)
        // Java: mpx.setBaselineWork(); (commented out)

        // Java: mpx.setBudget(BooleanHelper.getBoolean(xml.isIsBudget()));
        resource
            .borrow_mut()
            .set_budget(BooleanHelper::get_boolean(xml.is_budget));

        // Java: mpx.setCanLevel(BooleanHelper.getBoolean(xml.isCanLevel()));
        resource
            .borrow_mut()
            .set_can_level(BooleanHelper::get_boolean(xml.can_level));

        // Java: mpx.setCode(xml.getCode());
        resource.borrow_mut().set_code(xml.code.clone());

        // Java: mpx.setCost(DatatypeConverter.parseCurrency(xml.getCost()));
        resource
            .borrow_mut()
            .set_cost(DatatypeConverter::parse_currency(xml.cost));

        // Java: mpx.setCostCenter(xml.getCostCenter());
        resource.borrow_mut().set_cost_center(xml.cost_center.clone());

        // Java: mpx.setCostVariance(DatatypeConverter.parseCurrency(xml.getCostVariance()));
        resource
            .borrow_mut()
            .set_cost_variance(DatatypeConverter::parse_currency(xml.cost_variance));

        // Java: mpx.setCreationDate(xml.getCreationDate());
        resource.borrow_mut().set_creation_date(xml.creation_date);

        // Java: mpx.setCV(DatatypeConverter.parseCurrency(xml.getCV()));
        resource
            .borrow_mut()
            .set_cv(DatatypeConverter::parse_currency(xml.cv));

        // Java: mpx.setEmailAddress(xml.getEmailAddress());
        resource
            .borrow_mut()
            .set_email_address(xml.email_address.clone());

        // Java: mpx.setGroup(xml.getGroup());
        resource.borrow_mut().set_group(xml.group.clone());

        // Java: mpx.setGUID(xml.getGUID());
        resource.borrow_mut().set_guid(xml.guid);

        // Java: mpx.setHyperlink(xml.getHyperlink());
        resource.borrow_mut().set_hyperlink(xml.hyperlink.clone());

        // Java: mpx.setHyperlinkAddress(xml.getHyperlinkAddress());
        resource
            .borrow_mut()
            .set_hyperlink_address(xml.hyperlink_address.clone());

        // Java: mpx.setHyperlinkSubAddress(xml.getHyperlinkSubAddress());
        resource
            .borrow_mut()
            .set_hyperlink_sub_address(xml.hyperlink_sub_address.clone());

        // Java: mpx.setID(NumberHelper.getInteger(xml.getID()));
        resource
            .borrow_mut()
            .set_id(NumberHelper::get_integer(xml.id.map(|value| value as f64)));

        // Java: mpx.setInitials(xml.getInitials());
        resource.borrow_mut().set_initials(xml.initials.clone());

        // Java: mpx.setEnterprise(BooleanHelper.getBoolean(xml.isIsEnterprise()));
        resource
            .borrow_mut()
            .set_enterprise(BooleanHelper::get_boolean(xml.is_enterprise));

        // Java: mpx.setGeneric(BooleanHelper.getBoolean(xml.isIsGeneric()));
        resource
            .borrow_mut()
            .set_generic(BooleanHelper::get_boolean(xml.is_generic));

        // Java: mpx.setActive(!BooleanHelper.getBoolean(xml.isIsInactive()));
        resource
            .borrow_mut()
            .set_active(!BooleanHelper::get_boolean(xml.is_inactive));

        // Java: mpx.setIsNull(BooleanHelper.getBoolean(xml.isIsNull()));
        resource
            .borrow_mut()
            .set_is_null(BooleanHelper::get_boolean(xml.is_null));

        // Java: mpx.setLinkedFields(); (commented out)

        // Java: mpx.setUnitOfMeasure(m_projectFile.getUnitsOfMeasure().getOrCreateByAbbreviation(xml.getMaterialLabel()));
        let unit_of_measure = resources_unit_of_measure(&self.file, xml.material_label.as_deref());
        resource
            .borrow_mut()
            .set_unit_of_measure(unit_of_measure.as_ref());

        // Java: mpx.setName(xml.getName());
        resource.borrow_mut().set_name(xml.name.clone());

        // Java:
        // if (xml.getNotes() != null && !xml.getNotes().isEmpty())
        // {
        //    mpx.setNotes(xml.getNotes());
        // }
        if xml.notes.as_deref().is_some_and(|notes| !notes.is_empty()) {
            resource.borrow_mut().set_notes(xml.notes.clone());
        }

        // Java: mpx.setNtAccount(xml.getNTAccount());
        resource.borrow_mut().set_nt_account(xml.nt_account.clone());

        // Java: mpx.setObjects(); (commented out)

        // Java: mpx.setOvertimeCost(DatatypeConverter.parseCurrency(xml.getOvertimeCost()));
        resource
            .borrow_mut()
            .set_overtime_cost(DatatypeConverter::parse_currency(xml.overtime_cost));

        // Java: mpx.setOvertimeWork(DatatypeConverter.parseDuration(m_projectFile, null, xml.getOvertimeWork()));
        resource.borrow_mut().set_overtime_work(resources_parse_duration(
            &self.file,
            None,
            xml.overtime_work.as_deref(),
        ));

        // Java: mpx.setPeakUnits(DatatypeConverter.parseUnits(xml.getPeakUnits()));
        resource
            .borrow_mut()
            .set_peak_units(DatatypeConverter::parse_units(xml.peak_units));

        // Java: mpx.setPhonetics(xml.getPhonetics());
        resource.borrow_mut().set_phonetics(xml.phonetics.clone());

        // Java: mpx.setRegularWork(DatatypeConverter.parseDuration(m_projectFile, null, xml.getRegularWork()));
        resource.borrow_mut().set_regular_work(resources_parse_duration(
            &self.file,
            None,
            xml.regular_work.as_deref(),
        ));

        // Java: mpx.setRemainingCost(DatatypeConverter.parseCurrency(xml.getRemainingCost()));
        resource
            .borrow_mut()
            .set_remaining_cost(DatatypeConverter::parse_currency(xml.remaining_cost));

        // Java: mpx.setRemainingOvertimeCost(DatatypeConverter.parseCurrency(xml.getRemainingOvertimeCost()));
        resource
            .borrow_mut()
            .set_remaining_overtime_cost(DatatypeConverter::parse_currency(
                xml.remaining_overtime_cost,
            ));

        // Java: mpx.setRemainingOvertimeWork(DatatypeConverter.parseDuration(m_projectFile, null, xml.getRemainingOvertimeWork()));
        resource
            .borrow_mut()
            .set_remaining_overtime_work(resources_parse_duration(
                &self.file,
                None,
                xml.remaining_overtime_work.as_deref(),
            ));

        // Java: mpx.setSV(DatatypeConverter.parseCurrency(xml.getSV()));
        resource
            .borrow_mut()
            .set_sv(DatatypeConverter::parse_currency(xml.sv));

        // Java: mpx.setType(xml.getType());
        if let Some(resource_type) = xml.r#type {
            resource.borrow_mut().set_type(resource_type);
        }

        // Java: mpx.setUniqueID(NumberHelper.getInteger(xml.getUID()));
        resource
            .borrow_mut()
            .set_unique_id(NumberHelper::get_integer(xml.uid.map(f64::from)));

        // Java: mpx.setWork(DatatypeConverter.parseDuration(m_projectFile, null, xml.getWork()));
        resource
            .borrow_mut()
            .set_work(resources_parse_duration(&self.file, None, xml.work.as_deref()));

        // Java: mpx.setWorkGroup(xml.getWorkGroup());
        if let Some(work_group) = xml.work_group {
            resource.borrow_mut().set_work_group(work_group);
        }

        // Java: mpx.setWorkVariance(DatatypeConverter.parseDurationInThousanthsOfMinutes(xml.getWorkVariance()));
        resource
            .borrow_mut()
            .set_work_variance(DatatypeConverter::parse_duration_in_thousanths_of_minutes(
                xml.work_variance,
            ));

        // Java:
        // if (mpx.getType() == ResourceType.MATERIAL && BooleanHelper.getBoolean(xml.isIsCostResource()))
        // {
        //    mpx.setType(ResourceType.COST);
        // }
        let resource_type = resource.borrow().get_type();
        if resource_type == crate::resource_type::ResourceType::Material
            && BooleanHelper::get_boolean(xml.is_cost_resource)
        {
            resource
                .borrow_mut()
                .set_type(crate::resource_type::ResourceType::Cost);
        }

        // Java: mpx.setActualWork(DatatypeConverter.parseDuration(m_projectFile, null, xml.getActualWork()));
        resource.borrow_mut().set_actual_work(resources_parse_duration(
            &self.file,
            None,
            xml.actual_work.as_deref(),
        ));

        // Java: mpx.setRemainingWork(DatatypeConverter.parseDuration(m_projectFile, null, xml.getRemainingWork()));
        resource.borrow_mut().set_remaining_work(resources_parse_duration(
            &self.file,
            None,
            xml.remaining_work.as_deref(),
        ));

        // Java: mpx.setPercentWorkComplete(xml.getPercentWorkComplete());
        resource
            .borrow_mut()
            .set_percent_work_complete(xml.percent_work_complete);

        // Java: readResourceExtendedAttributes(xml, mpx);
        self.read_resource_extended_attributes(xml, &resource);

        // Java: readResourceOutlineCodes(xml, mpx);
        self.read_resource_outline_codes(xml, &resource);

        // Java: readResourceBaselines(xml, mpx);
        self.read_resource_baselines(xml, &resource);

        // Java: mpx.setCalendar(calendarMap.get(xml.getCalendarUID()));
        let calendar = xml
            .calendar_uid
            .and_then(|unique_id| calendar_map.get(&unique_id))
            .cloned();
        resource.borrow_mut().set_calendar(calendar.as_ref());

        // Java: Rate standardRate = DatatypeConverter.parseRate(xml.getStandardRate(), DatatypeConverter.parseTimeUnit(xml.getStandardRateFormat()));
        let standard_rate = resources_parse_rate(
            &self.file,
            xml.standard_rate,
            resources_time_unit(xml.standard_rate_format.map(|value| value as f64)),
        );

        // Java: Rate overtimeRate = DatatypeConverter.parseRate(xml.getOvertimeRate(), DatatypeConverter.parseTimeUnit(xml.getOvertimeRateFormat()));
        let overtime_rate = resources_parse_rate(
            &self.file,
            xml.overtime_rate,
            resources_time_unit(xml.overtime_rate_format.map(|value| value as f64)),
        );

        // Java: Number costPerUse = DatatypeConverter.parseCurrency(xml.getCostPerUse());
        let cost_per_use = DatatypeConverter::parse_currency(xml.cost_per_use);

        // Java: readCostRateTables(mpx, standardRate, overtimeRate, costPerUse, xml.getRates());
        self.read_cost_rate_tables(
            &resource,
            standard_rate,
            overtime_rate,
            cost_per_use,
            xml.rates_present.then_some(xml.rates.as_slice()),
        );

        // Java: readAvailabilityTable(mpx, xml);
        self.read_availability_table(&resource, xml);

        // Java: ensure that we cache this value
        //       mpx.setOverAllocated(BooleanHelper.getBoolean(xml.isOverAllocated()));
        resource
            .borrow_mut()
            .set_over_allocated(BooleanHelper::get_boolean(xml.over_allocated));

        // Java: m_eventManager.fireResourceReadEvent(mpx);
        self.with_event_manager(|manager| manager.fire_resource_read_event(&resource));
    }

    /// Reads baseline values for the current resource.
    ///
    /// Corresponds to Java: `private void readResourceBaselines(Project.Resources.Resource xmlResource, Resource mpxjResource)`
    fn read_resource_baselines(&mut self, xml_resource: &XmlResource, mpxj_resource: &Rc<RefCell<Resource>>) {
        for baseline in &xml_resource.baseline {
            // Java: int number = NumberHelper.getInt(baseline.getNumber());
            let number = NumberHelper::get_int(baseline.number.map(|value| value as f64));

            // Java: Double cost = DatatypeConverter.parseCurrency(baseline.getCost());
            let cost = DatatypeConverter::parse_currency(baseline.cost);

            // Java: Duration work = DatatypeConverter.parseDuration(m_projectFile, TimeUnit.HOURS, baseline.getWork());
            let work = resources_parse_duration(
                &self.file,
                Some(TimeUnit::Hours),
                baseline.work.as_deref(),
            );

            if number == 0 {
                // Java: mpxjResource.setBaselineCost(cost); mpxjResource.setBaselineWork(work);
                mpxj_resource.borrow_mut().set_baseline_cost(cost);
                mpxj_resource.borrow_mut().set_baseline_work(work);
            } else {
                // Java: mpxjResource.setBaselineCost(number, cost); mpxjResource.setBaselineWork(number, work);
                mpxj_resource.borrow_mut().set_baseline_cost_at(number, cost);
                mpxj_resource.borrow_mut().set_baseline_work_at(number, work);
            }
        }
    }

    /// This method processes any extended attributes associated with a resource.
    ///
    /// Corresponds to Java: `private void readResourceExtendedAttributes(Project.Resources.Resource xml, Resource mpx)`
    fn read_resource_extended_attributes(&mut self, xml: &XmlResource, resource: &Rc<RefCell<Resource>>) {
        for attrib in &xml.extended_attribute {
            // Java: FieldType mpxFieldID = FieldTypeHelper.getInstance(m_projectFile, Integer.parseInt(attrib.getFieldID()));
            // An unparsable FieldID throws NumberFormatException in Java; the Rust side skips it,
            // and an unknown ID yields None, which `parseCustomField` ignores (Java's null field).
            let field_id = attrib
                .field_id
                .as_deref()
                .and_then(|value| value.parse::<i32>().ok())
                .and_then(|value| FieldTypeHelper::get_instance(&self.file, value))
                .map(|field| field.to_field_key());

            // Java: TimeUnit durationFormat = DatatypeConverter.parseDurationTimeUnits(attrib.getDurationFormat(), null);
            let duration_format =
                resources_parse_duration_time_units(&self.file, attrib.duration_format);

            // Java: DatatypeConverter.parseCustomField(m_projectFile, mpx, attrib.getValue(), mpxFieldID, durationFormat);
            DatatypeConverter::parse_custom_field(
                &self.file,
                self.ignore_errors,
                &mut *resource.borrow_mut(),
                attrib.value.as_deref(),
                field_id,
                duration_format,
            );
        }
    }

    /// This method processes any outline codes associated with a resource.
    ///
    /// Corresponds to Java: `private void readResourceOutlineCodes(Project.Resources.Resource xml, Resource mpx)`
    fn read_resource_outline_codes(&mut self, xml: &XmlResource, resource: &Rc<RefCell<Resource>>) {
        for attrib in &xml.outline_code {
            // Java:
            // if (attrib.getFieldID() == null)
            // {
            //    continue;
            // }
            let Some(field_id) = attrib.field_id.as_deref() else {
                continue;
            };

            // Java: FieldType mpxFieldID = FieldTypeHelper.getInstance(m_projectFile, Integer.parseInt(attrib.getFieldID()));
            let field_key = field_id
                .parse::<i32>()
                .ok()
                .and_then(|value| FieldTypeHelper::get_instance(&self.file, value))
                .map(|field| field.to_field_key());

            // Rust-specific branch: `Resource::set` needs a concrete field key, so an unparsable or
            // unknown field ID skips the record (Java passes `null` to `set`, which stores nothing).
            let Some(field_key) = field_key else {
                continue;
            };

            // Java: mpx.set(mpxFieldID, getOutlineCodeValue(mpxFieldID, attrib.getValueID()));
            let value = self.get_outline_code_value(Some(field_key), attrib.value_id);
            resource
                .borrow_mut()
                .set(field_key, value.map(FieldValue::String));
        }
    }

    /// Reads the cost rate tables from the file.
    ///
    /// Corresponds to Java: `private void readCostRateTables(Resource resource, Rate defaultStandardRate, Rate defaultOvertimeRate, Number defaultCostPerUse, Rates rates)`
    ///
    /// `rates` is the flattened slice of the Java `Rates` wrapper: an empty slice plays the role of
    /// Java's `rates == null` (see the file header).
    fn read_cost_rate_tables(
        &mut self,
        resource: &Rc<RefCell<Resource>>,
        default_standard_rate: Option<Rate>,
        default_overtime_rate: Option<Rate>,
        default_cost_per_use: Option<f64>,
        rates: Option<&[XmlRate]>,
    ) {
        let Some(rates) = rates else {
            for index in 0..CostRateTable::MAX_TABLES {
                let mut table = CostRateTable::new();
                if index == 0 {
                    // Java:
                    // LocalDateTime startDate = CostRateTableEntry.DEFAULT_ENTRY.getStartDate();
                    // LocalDateTime endDate = CostRateTableEntry.DEFAULT_ENTRY.getEndDate();
                    // table.add(new CostRateTableEntry(startDate, endDate, defaultCostPerUse, defaultStandardRate, defaultOvertimeRate));
                    let default_entry = CostRateTableEntry::default_entry();
                    table.add(CostRateTableEntry::new_with_optional_rates(
                        default_entry.get_start_date(),
                        default_entry.get_end_date(),
                        default_cost_per_use,
                        None,
                        &[default_standard_rate, default_overtime_rate],
                    ));
                } else {
                    // Java: table.add(CostRateTableEntry.DEFAULT_ENTRY);
                    table.add(CostRateTableEntry::default_entry());
                }

                // Java: resource.setCostRateTable(index, table);
                resource.borrow_mut().set_cost_rate_table(index, table);
            }

            return;
        };

        // Java: CostRateTable[] tables = new CostRateTable[CostRateTable.MAX_TABLES];
        // The Rust side collects the entries per table index; an empty vector is Java's null slot
        // (a table is only created when an entry is added, see the file header).
        let mut tables: Vec<Vec<CostRateTableEntry>> = (0..CostRateTable::MAX_TABLES)
            .map(|_| Vec::new())
            .collect();

        for rate in rates {
            // Java: if (rate.getRateTable() == null) { continue; }
            let Some(rate_table) = rate.rate_table else {
                continue;
            };

            // Java: int tableIndex = rate.getRateTable().intValue();
            //       if (tableIndex < 0 || tableIndex >= CostRateTable.MAX_TABLES) { continue; }
            let table_index = rate_table as i32;
            if table_index < 0 || table_index >= CostRateTable::MAX_TABLES {
                continue;
            }

            // Java: TimeUnit standardRateFormat = DatatypeConverter.parseTimeUnit(rate.getStandardRateFormat());
            //       Rate standardRate = DatatypeConverter.parseRate(rate.getStandardRate(), standardRateFormat);
            let standard_rate = resources_parse_rate(
                &self.file,
                rate.standard_rate,
                resources_time_unit(rate.standard_rate_format.map(|value| value as f64)),
            );

            // Java: TimeUnit overtimeRateFormat = DatatypeConverter.parseTimeUnit(rate.getOvertimeRateFormat());
            //       Rate overtimeRate = DatatypeConverter.parseRate(rate.getOvertimeRate(), overtimeRateFormat);
            let overtime_rate = resources_parse_rate(
                &self.file,
                rate.overtime_rate,
                resources_time_unit(rate.overtime_rate_format.map(|value| value as f64)),
            );

            // Java: Double costPerUse = DatatypeConverter.parseCurrency(rate.getCostPerUse());
            let cost_per_use = DatatypeConverter::parse_currency(rate.cost_per_use);

            // Java: LocalDateTime startDate = rate.getRatesFrom();
            //       LocalDateTime endDate = rate.getRatesTo();
            let start_date = rate.rates_from.map(|start_date| {
                if start_date < LocalDateTimeHelper::start_date_na() {
                    LocalDateTimeHelper::start_date_na()
                } else {
                    start_date
                }
            });

            let end_date = rate.rates_to.map(|end_date| {
                let mut end_date = if end_date > LocalDateTimeHelper::end_date_na() {
                    LocalDateTimeHelper::end_date_na()
                } else {
                    end_date
                };

                // Java:
                // //
                // // See the note in CostRateTableFactory for more details of this heuristic.
                // //
                // int minutes = endDate.getMinute();
                // if ((minutes % 5) == 0)
                // {
                //    endDate = endDate.minusMinutes(1);
                // }
                if end_date.minute() % 5 == 0 {
                    // `checked_sub_signed` keeps the subtraction from panicking at the start of
                    // chrono's range, where Java's `minusMinutes` would throw (Rust-specific branch).
                    end_date = end_date
                        .checked_sub_signed(ChronoDuration::minutes(1))
                        .unwrap_or(end_date);
                }

                end_date
            });

            // Java:
            // CostRateTableEntry entry = new CostRateTableEntry(startDate, endDate, costPerUse, standardRate, overtimeRate);
            // CostRateTable table = tables[tableIndex];
            // if (table == null) { table = new CostRateTable(); tables[tableIndex] = table; }
            // table.add(entry);
            tables[table_index as usize].push(CostRateTableEntry::new_with_optional_rates(
                start_date,
                end_date,
                cost_per_use,
                None,
                &[standard_rate, overtime_rate],
            ));
        }

        // Java:
        // for (int tableIndex = 0; tableIndex < tables.length; tableIndex++)
        // {
        //    CostRateTable table = tables[tableIndex];
        //    if (table != null)
        //    {
        //       Collections.sort(table);
        //       resource.setCostRateTable(tableIndex, table);
        //    }
        // }
        for (table_index, entries) in tables.into_iter().enumerate() {
            if entries.is_empty() {
                continue;
            }

            let mut entries = entries;
            entries.sort();
            resource.borrow_mut().set_cost_rate_table(
                table_index as i32,
                CostRateTable::from_entries(entries),
            );
        }
    }

    /// Reads the availability table from the file.
    ///
    /// Corresponds to Java: `private void readAvailabilityTable(Resource resource, Project.Resources.Resource xml)`
    fn read_availability_table(&mut self, resource: &Rc<RefCell<Resource>>, xml: &XmlResource) {
        // Java:
        // AvailabilityPeriods periods = xml.getAvailabilityPeriods();
        // if (periods == null || periods.getAvailabilityPeriod().isEmpty())
        // {
        //    LocalDateTime availableFrom = xml.getAvailableFrom();
        //    LocalDateTime availableTo = xml.getAvailableTo();
        //    availableFrom = availableFrom == null ? LocalDateTimeHelper.START_DATE_NA : availableFrom;
        //    availableTo = availableTo == null ? LocalDateTimeHelper.END_DATE_NA : availableTo;
        //    resource.getAvailability().add(new Availability(availableFrom, availableTo, DatatypeConverter.parseUnits(xml.getMaxUnits())));
        // }
        if xml.availability_periods.is_empty() {
            let available_from = xml
                .available_from
                .unwrap_or_else(LocalDateTimeHelper::start_date_na);
            let available_to = xml
                .available_to
                .unwrap_or_else(LocalDateTimeHelper::end_date_na);

            resource.borrow_mut().add_availability(Availability::new(
                Some(available_from),
                Some(available_to),
                DatatypeConverter::parse_units(xml.max_units),
            ));

            return;
        }

        // Java:
        // AvailabilityTable table = resource.getAvailability();
        // List<AvailabilityPeriod> list = periods.getAvailabilityPeriod();
        // for (AvailabilityPeriod period : list)
        // {
        //    LocalDateTime start = period.getAvailableFrom();
        //    LocalDateTime end = period.getAvailableTo();
        //    Number units = DatatypeConverter.parseUnits(period.getAvailableUnits());
        //
        //    if (start == null || start.isBefore(LocalDateTimeHelper.START_DATE_NA)) { start = LocalDateTimeHelper.START_DATE_NA; }
        //    if (end == null || end.isAfter(LocalDateTimeHelper.END_DATE_NA)) { end = LocalDateTimeHelper.END_DATE_NA; }
        //
        //    Availability availability = new Availability(start, end, units);
        //    table.add(availability);
        // }
        // Collections.sort(table);
        //
        // Rust-specific: the entries are collected first, sorted, and then appended through
        // `Resource::add_availability` (see the file header).
        let mut entries: Vec<Availability> = Vec::new();

        for period in &xml.availability_periods {
            let start = match period.available_from {
                Some(start) if start >= LocalDateTimeHelper::start_date_na() => Some(start),
                _ => Some(LocalDateTimeHelper::start_date_na()),
            };

            let end = match period.available_to {
                Some(end) if end <= LocalDateTimeHelper::end_date_na() => Some(end),
                _ => Some(LocalDateTimeHelper::end_date_na()),
            };

            let units = DatatypeConverter::parse_units(period.available_units);

            entries.push(Availability::new(start, end, units));
        }

        entries.sort();

        let mut resource = resource.borrow_mut();
        for entry in entries {
            resource.add_availability(entry);
        }
    }
}

/// Rust-specific helper: `DatatypeConverter.parseDuration(m_projectFile, defaultUnits, value)`.
///
/// Corresponds to Java: `DatatypeConverter.parseDuration(ProjectFile file, TimeUnit defaultUnits, String value)`
///
/// The frozen Rust `parse_duration` takes `&ProjectFile`, so the file borrow is opened here; a
/// borrow conflict degrades to `None` (a Rust-specific branch that cannot happen on the call paths
/// of this module, which never hold a borrow of the file).
fn resources_parse_duration(
    file: &Rc<RefCell<ProjectFile>>,
    default_units: Option<TimeUnit>,
    value: Option<&str>,
) -> Option<Duration> {
    match file.try_borrow() {
        Ok(file) => DatatypeConverter::parse_duration(&file, default_units, value),
        Err(_) => None,
    }
}

/// Rust-specific helper: `DatatypeConverter.parseDurationTimeUnits(value, null)`.
///
/// Corresponds to Java: `DatatypeConverter.parseDurationTimeUnits(BigInteger value, TimeUnit defaultValue)`
///
/// Java's two-argument overload returns the supplied default for a null value (the `null` used at
/// this call site) and, for a value that is not in the dictionary, the parent file's default
/// duration units. [`DatatypeConverter::parse_duration_time_units_with_default`] carries no file
/// and [`DatatypeConverter::parse_duration_time_units`] turns a null value into `HOURS`, so the two
/// halves are combined here.
fn resources_parse_duration_time_units(
    file: &Rc<RefCell<ProjectFile>>,
    value: Option<i64>,
) -> Option<TimeUnit> {
    if value.is_none() {
        return None;
    }

    match file.try_borrow() {
        Ok(file) => DatatypeConverter::parse_duration_time_units(value, &file),
        Err(_) => DatatypeConverter::parse_duration_time_units_with_default(value, None),
    }
}

/// Rust-specific helper: `DatatypeConverter.parseTimeUnit(value)`.
///
/// Corresponds to Java: `DatatypeConverter.parseTimeUnit(Number value)`
///
/// Java never returns `null`; the helper resolves the (unreachable) `None` to `DAYS`, which is
/// Java's own fallback for a value outside the dictionary.
fn resources_time_unit(value: Option<f64>) -> TimeUnit {
    DatatypeConverter::parse_time_unit(value).unwrap_or(TimeUnit::Days)
}

/// Rust-specific helper: `DatatypeConverter.parseRate(value, targetUnits)` with the parent file's
/// `ProjectProperties`.
///
/// Corresponds to Java: `DatatypeConverter.parseRate(BigDecimal originalValue, TimeUnit targetUnits)`
///
/// Java reads the properties from the `PARENT_FILE` `ThreadLocal`; the Rust port reads them from the
/// file, which is the faithful call-site choice (see the file header). A borrow conflict degrades
/// to `None` (Rust-specific branch, unreachable on this module's call paths).
fn resources_parse_rate(
    file: &Rc<RefCell<ProjectFile>>,
    original_value: Option<f64>,
    target_units: TimeUnit,
) -> Option<Rate> {
    match file.try_borrow() {
        Ok(file) => DatatypeConverter::parse_rate_with_properties(
            file.get_project_properties(),
            original_value,
            target_units,
        ),
        Err(_) => None,
    }
}

/// Rust-specific helper:
/// `m_projectFile.getUnitsOfMeasure().getOrCreateByAbbreviation(abbreviation)`.
///
/// Corresponds to Java: `public UnitOfMeasure getOrCreateByAbbreviation(String abbreviation)`
/// (`UnitOfMeasureContainer`)
///
/// The container lives in the `ProjectContext`, so it is borrowed mutably to create a missing entry;
/// the entry is cloned so that the borrow ends before the resource is borrowed (see the file
/// header). A missing abbreviation yields `None`, as in Java.
fn resources_unit_of_measure(
    file: &Rc<RefCell<ProjectFile>>,
    abbreviation: Option<&str>,
) -> Option<UnitOfMeasure> {
    let context = file.try_borrow().ok()?.get_project_context();
    let mut context = context.try_borrow_mut().ok()?;

    context
        .get_units_of_measure_mut()
        .get_or_create_by_abbreviation(abbreviation)
        .cloned()
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::io::Cursor;
    use std::rc::Rc;

    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

    use crate::common::local_date_time_helper::LocalDateTimeHelper;
    use crate::duration::Duration;
    use crate::field_key::FieldKey;
    use crate::field_value::FieldValue;
    use crate::mspdi::mspdi_reader::MSPDIReader;
    use crate::project_entity_with_id::ProjectEntityWithID;
    use crate::project_entity_with_unique_id::ProjectEntityWithUniqueID;
    use crate::project_file::ProjectFile;
    use crate::rate::Rate;
    use crate::resource::Resource;
    use crate::resource_field::ResourceField;
    use crate::resource_type::ResourceType;
    use crate::time_unit::TimeUnit;

    /// The `<Resource>` elements of the documents below are embedded in a document whose duration
    /// format is `DAYS` (7 in the MSPDI dictionary) and whose working time is the library default
    /// (480 minutes per day), so that the `xsd:duration` conversions are easy to predict.
    fn document(resources: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Project xmlns="http://schemas.microsoft.com/project">
    <DurationFormat>7</DurationFormat>
    <MinutesPerDay>480</MinutesPerDay>
    <MinutesPerWeek>2400</MinutesPerWeek>
    <Resources>{resources}</Resources>
</Project>"#
        )
    }

    /// Read a document holding a single resource and return the project file together with that
    /// resource.
    ///
    /// The file must be kept alive by the caller: a resource only holds a weak reference to its
    /// `ProjectContext` (Java's `m_projectFile` is a strong reference), so values reached through the
    /// context — the unit of measure, for example — become unavailable once the file is dropped.
    fn read_resource(resource_xml: &str) -> (Rc<RefCell<ProjectFile>>, Rc<RefCell<Resource>>) {
        let xml = document(resource_xml);
        let mut reader = MSPDIReader::new();
        let file = reader
            .read(&mut Cursor::new(xml.as_bytes()))
            .expect("the document should be read");

        let resources: Vec<Rc<RefCell<Resource>>> =
            file.borrow().get_resources().iter().cloned().collect();
        assert_eq!(resources.len(), 1, "the fixture holds exactly one resource");
        (file, resources[0].clone())
    }

    /// Build a date-time for the assertions.
    fn date_time(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: u32) -> NaiveDateTime {
        NaiveDateTime::new(
            NaiveDate::from_ymd_opt(year, month, day).expect("valid date"),
            NaiveTime::from_hms_opt(hour, minute, second).expect("valid time"),
        )
    }

    /// The scalar setters convert their values exactly like Java (currency is divided by 100, the
    /// `xsd:duration` values are converted with the project's default duration units, and the
    /// nullable enums are only written when the element is present).
    #[test]
    fn reads_the_scalar_fields_of_a_resource() {
        let (_file, resource) = read_resource(
            r#"<Resource>
                <UID>1</UID>
                <ID>10</ID>
                <Name>Resource One</Name>
                <Type>1</Type>
                <Initials>R1</Initials>
                <EmailAddress>r1@example.com</EmailAddress>
                <Code>100</Code>
                <Group>Steelray</Group>
                <WorkGroup>2</WorkGroup>
                <AccrueAt>2</AccrueAt>
                <BookingType>1</BookingType>
                <MaterialLabel>kg</MaterialLabel>
                <Notes>Resource notes</Notes>
                <Work>PT80H0M0S</Work>
                <RegularWork>PT40H0M0S</RegularWork>
                <Cost>10025</Cost>
                <CostPerUse>350</CostPerUse>
                <PercentWorkComplete>25.5</PercentWorkComplete>
                <CreationDate>2010-03-16T21:57:00</CreationDate>
                <IsGeneric>1</IsGeneric>
                <IsEnterprise>1</IsEnterprise>
                <IsInactive>1</IsInactive>
                <IsNull>0</IsNull>
                <IsBudget>1</IsBudget>
                <CanLevel>1</CanLevel>
                <OverAllocated>1</OverAllocated>
            </Resource>"#,
        );

        let resource = resource.borrow();

        assert_eq!(resource.get_unique_id(), Some(1));
        assert_eq!(resource.get_id(), Some(10));
        assert_eq!(resource.get_name().as_deref(), Some("Resource One"));
        assert_eq!(resource.get_type(), ResourceType::Work);
        assert_eq!(resource.get_initials().as_deref(), Some("R1"));
        assert_eq!(resource.get_email_address().as_deref(), Some("r1@example.com"));
        assert_eq!(resource.get_code().as_deref(), Some("100"));
        assert_eq!(resource.get_group().as_deref(), Some("Steelray"));
        assert_eq!(resource.get_work_group(), Some(crate::work_group::WorkGroup::Email));
        assert_eq!(resource.get_accrue_at(), Some(crate::accrue_type::AccrueType::End));
        assert_eq!(
            resource.get_booking_type(),
            Some(crate::booking_type::BookingType::Proposed)
        );
        assert_eq!(resource.get_notes(), "Resource notes");
        // 80 hours is 10 days at 480 minutes per day.
        assert_eq!(resource.get_work(), Some(Duration::get_instance(10.0, TimeUnit::Days)));
        assert_eq!(
            resource.get_regular_work(),
            Some(Duration::get_instance(5.0, TimeUnit::Days))
        );
        // Currency values are divided by 100.
        assert_eq!(resource.get_cost(), Some(100.25));
        assert_eq!(resource.get_cost_per_use(), Some(3.5));
        assert_eq!(resource.get_percent_work_complete(), Some(25.5));
        assert_eq!(
            resource.get_creation_date(),
            Some(date_time(2010, 3, 16, 21, 57, 0))
        );
        assert!(resource.get_generic());
        assert!(resource.get_enterprise());
        // Java: setActive(!BooleanHelper.getBoolean(xml.isIsInactive()))
        assert!(!resource.get_active());
        assert!(!resource.get_null());
        assert!(resource.get_budget());
        assert!(resource.get_can_level());
        assert!(resource.get_over_allocated());
        // The material label creates a unit of measure and associates it with the resource.
        let unit_of_measure = resource
            .get_unit_of_measure()
            .expect("the material label should create a unit of measure");
        assert_eq!(unit_of_measure.get_abbreviation(), Some("kg"));
    }

    /// Java promotes a *material* resource carrying `<IsCostResource>1</IsCostResource>` to `COST`
    /// (this is the only promotion in `readResource`); a work resource is left alone.
    #[test]
    fn promotes_a_cost_resource() {
        let (_file, material) = read_resource(
            "<Resource><UID>1</UID><Type>0</Type><IsCostResource>1</IsCostResource></Resource>",
        );
        assert_eq!(material.borrow().get_type(), ResourceType::Cost);

        let (_file, work) = read_resource(
            "<Resource><UID>2</UID><Type>1</Type><IsCostResource>1</IsCostResource></Resource>",
        );
        assert_eq!(work.borrow().get_type(), ResourceType::Work);

        let (_file, plain) = read_resource("<Resource><UID>3</UID><Type>0</Type></Resource>");
        assert_eq!(plain.borrow().get_type(), ResourceType::Material);
    }

    /// A resource element that carries nothing but a UID leaves every optional field unset.
    #[test]
    fn reads_a_resource_without_optional_fields() {
        let (_file, resource) = read_resource("<Resource><UID>1</UID></Resource>");
        let resource = resource.borrow();

        assert_eq!(resource.get_unique_id(), Some(1));
        assert_eq!(resource.get_id(), None);
        assert_eq!(resource.get_name(), None);
        assert_eq!(resource.get_initials(), None);
        assert_eq!(resource.get_email_address(), None);
        assert_eq!(resource.get_notes(), "");
        assert_eq!(resource.get_work(), None);
        assert_eq!(resource.get_percent_work_complete(), None);
        assert_eq!(resource.get_cost(), None);
        // The nullable enum fields stay unset.
        assert_eq!(resource.get_accrue_at(), None);
        assert_eq!(resource.get_work_group(), None);
        assert_eq!(resource.get_booking_type(), None);
        assert!(resource.get_unit_of_measure().is_none());
        // Java's BooleanHelper reads an absent Boolean as false.
        assert!(!resource.get_over_allocated());
        assert!(!resource.get_generic());
        assert!(!resource.get_enterprise());
        assert!(!resource.get_budget());
        assert!(!resource.get_can_level());
        // Java: setActive(!getBoolean(null)) -> true
        assert!(resource.get_active());
        assert_eq!(resource.get_creation_date(), None);

        // No `<AvailabilityPeriods>`: the table holds the `<AvailableFrom>`/`<AvailableTo>` range
        // with the units taken from `<MaxUnits>` (absent here, so the units are null).
        let availability = resource.get_availability();
        assert_eq!(availability.len(), 1);
        let entry = availability.get(0).expect("one availability row");
        assert_eq!(
            entry.get_range().get_start(),
            Some(LocalDateTimeHelper::start_date_na())
        );
        assert_eq!(entry.get_range().get_end(), Some(LocalDateTimeHelper::end_date_na()));
        assert_eq!(entry.get_units(), None);
    }

    /// The `Rates` elements are read into the resource's cost rate tables: the date range is
    /// clamped to the NA range, the minute heuristic runs on the end date, and the rate is
    /// converted from "amount per hour" to the format recorded in `StandardRateFormat`.
    #[test]
    fn reads_the_cost_rate_tables() {
        let (_file, resource) = read_resource(
            r#"<Resource>
                <UID>2</UID>
                <Name>Resource Two</Name>
                <Rates>
                    <Rate>
                        <RatesFrom>2020-01-01T00:00:00</RatesFrom>
                        <RatesTo>2021-01-01T00:00:00</RatesTo>
                        <RateTable>0</RateTable>
                        <StandardRate>25</StandardRate>
                        <StandardRateFormat>2</StandardRateFormat>
                        <OvertimeRate>40</OvertimeRate>
                        <OvertimeRateFormat>2</OvertimeRateFormat>
                        <CostPerUse>1500</CostPerUse>
                    </Rate>
                    <Rate>
                        <RatesFrom>2021-01-01T00:00:00</RatesFrom>
                        <RatesTo>2022-01-01T00:00:00</RatesTo>
                        <RateTable>1</RateTable>
                        <StandardRate>30</StandardRate>
                        <StandardRateFormat>3</StandardRateFormat>
                    </Rate>
                </Rates>
            </Resource>"#,
        );

        let resource = resource.borrow();

        // Table A: per hour, cost per use 1500 / 100 = 15.
        let table = resource.get_cost_rate_table(0).expect("table A");
        assert_eq!(table.len(), 1);
        let entry = table.get(0).expect("table A entry");
        assert_eq!(entry.get_start_date(), Some(date_time(2020, 1, 1, 0, 0, 0)));
        // 2021-01-01T00:00 has a minute value divisible by five, so a minute is subtracted.
        assert_eq!(entry.get_end_date(), Some(date_time(2020, 12, 31, 23, 59, 0)));
        assert_eq!(entry.get_cost_per_use(), Some(15.0));
        assert_eq!(entry.get_standard_rate(), Rate::new(25.0, TimeUnit::Hours));
        assert_eq!(entry.get_overtime_rate(), Rate::new(40.0, TimeUnit::Hours));

        // Table B: format 3 is per day, so 30 per hour is 30 * 480 / 60 = 240 per day.
        let table = resource.get_cost_rate_table(1).expect("table B");
        assert_eq!(table.len(), 1);
        let entry = table.get(0).expect("table B entry");
        assert_eq!(entry.get_start_date(), Some(date_time(2021, 1, 1, 0, 0, 0)));
        assert_eq!(entry.get_end_date(), Some(date_time(2021, 12, 31, 23, 59, 0)));
        assert_eq!(entry.get_cost_per_use(), None);
        assert_eq!(entry.get_standard_rate(), Rate::new(240.0, TimeUnit::Days));
        // An absent overtime rate is Java's null rate, represented as a zero rate in Rust.
        assert_eq!(entry.get_overtime_rate(), Rate::ZERO);

        // The tables that the file does not mention keep the default entry written by the
        // `Resource` constructor.
        let table = resource.get_cost_rate_table(2).expect("table C");
        assert_eq!(table.len(), 1);
        assert_eq!(table.get(0).map(|entry| entry.get_standard_rate()), Some(Rate::ZERO));
        assert_eq!(table.get(0).and_then(|entry| entry.get_cost_per_use()), Some(0.0));
    }

    /// Without a `Rates` wrapper all five tables are written: table A holds the resource's own
    /// rates, the other four hold the default entry.
    #[test]
    fn writes_the_default_cost_rate_tables() {
        let (_file, resource) = read_resource(
            r#"<Resource>
                <UID>2</UID>
                <StandardRate>25</StandardRate>
                <StandardRateFormat>2</StandardRateFormat>
                <OvertimeRate>40</OvertimeRate>
                <OvertimeRateFormat>2</OvertimeRateFormat>
                <CostPerUse>1500</CostPerUse>
            </Resource>"#,
        );

        let resource = resource.borrow();

        let table = resource.get_cost_rate_table(0).expect("table A");
        assert_eq!(table.len(), 1);
        let entry = table.get(0).expect("table A entry");
        assert_eq!(
            entry.get_start_date(),
            Some(LocalDateTimeHelper::start_date_na())
        );
        assert_eq!(entry.get_end_date(), Some(LocalDateTimeHelper::end_date_na()));
        assert_eq!(entry.get_cost_per_use(), Some(15.0));
        assert_eq!(entry.get_standard_rate(), Rate::new(25.0, TimeUnit::Hours));
        assert_eq!(entry.get_overtime_rate(), Rate::new(40.0, TimeUnit::Hours));

        for index in 1..crate::cost_rate_table::CostRateTable::MAX_TABLES {
            let table = resource
                .get_cost_rate_table(index)
                .expect("a default cost rate table");
            assert_eq!(table.len(), 1);
            let entry = table.get(0).expect("the default entry");
            assert_eq!(
                entry.get_start_date(),
                Some(LocalDateTimeHelper::start_date_na())
            );
            assert_eq!(entry.get_end_date(), Some(LocalDateTimeHelper::end_date_na()));
            assert_eq!(entry.get_cost_per_use(), Some(0.0));
            assert_eq!(entry.get_standard_rate(), Rate::ZERO);
            assert_eq!(entry.get_overtime_rate(), Rate::ZERO);
        }
    }

    /// The `AvailabilityPeriods` wrapper is read entry by entry: the dates are clamped to the NA
    /// range and the units are multiplied by 100, then the table is sorted by range.
    #[test]
    fn reads_the_availability_periods() {
        let (_file, resource) = read_resource(
            r#"<Resource>
                <UID>3</UID>
                <Name>Resource Two</Name>
                <MaxUnits>1</MaxUnits>
                <AvailableFrom>2009-06-01T00:00:00</AvailableFrom>
                <AvailableTo>2009-07-01T23:59:00</AvailableTo>
                <AvailabilityPeriods>
                    <AvailabilityPeriod>
                        <AvailableFrom>2009-06-01T00:00:00</AvailableFrom>
                        <AvailableTo>2009-07-01T23:59:00</AvailableTo>
                        <AvailableUnits>1</AvailableUnits>
                    </AvailabilityPeriod>
                    <AvailabilityPeriod>
                        <AvailableFrom>2009-07-02T00:00:00</AvailableFrom>
                        <AvailableTo>2009-08-01T23:59:00</AvailableTo>
                        <AvailableUnits>0.6</AvailableUnits>
                    </AvailabilityPeriod>
                </AvailabilityPeriods>
            </Resource>"#,
        );

        let resource = resource.borrow();
        let availability = resource.get_availability();

        assert_eq!(availability.len(), 2);

        let entry = availability.get(0).expect("the first period");
        assert_eq!(
            entry.get_range().get_start(),
            Some(date_time(2009, 6, 1, 0, 0, 0))
        );
        assert_eq!(entry.get_range().get_end(), Some(date_time(2009, 7, 1, 23, 59, 0)));
        assert_eq!(entry.get_units(), Some(100.0));

        let entry = availability.get(1).expect("the second period");
        assert_eq!(
            entry.get_range().get_start(),
            Some(date_time(2009, 7, 2, 0, 0, 0))
        );
        assert_eq!(entry.get_range().get_end(), Some(date_time(2009, 8, 1, 23, 59, 0)));
        assert_eq!(entry.get_units(), Some(60.0));
    }

    /// Without the `AvailabilityPeriods` wrapper the resource's `<AvailableFrom>`, `<AvailableTo>`
    /// and `<MaxUnits>` provide the single availability row.
    #[test]
    fn reads_the_availability_range_of_a_resource() {
        let (_file, resource) = read_resource(
            r#"<Resource>
                <UID>4</UID>
                <MaxUnits>1</MaxUnits>
                <AvailableFrom>2009-06-01T00:00:00</AvailableFrom>
                <AvailableTo>2009-07-01T23:59:00</AvailableTo>
            </Resource>"#,
        );

        let resource = resource.borrow();
        let availability = resource.get_availability();

        assert_eq!(availability.len(), 1);
        let entry = availability.get(0).expect("the availability row");
        assert_eq!(
            entry.get_range().get_start(),
            Some(date_time(2009, 6, 1, 0, 0, 0))
        );
        assert_eq!(entry.get_range().get_end(), Some(date_time(2009, 7, 1, 23, 59, 0)));
        assert_eq!(entry.get_units(), Some(100.0));
    }

    /// Baseline 0 is written to the plain baseline fields, other baselines to the numbered ones.
    #[test]
    fn reads_the_resource_baselines() {
        let (_file, resource) = read_resource(
            r#"<Resource>
                <UID>1</UID>
                <Baseline>
                    <Number>0</Number>
                    <Work>PT120H0M0S</Work>
                    <Cost>2000.5</Cost>
                </Baseline>
                <Baseline>
                    <Number>1</Number>
                    <Work>PT40H0M0S</Work>
                    <Cost>100</Cost>
                </Baseline>
            </Resource>"#,
        );

        let resource = resource.borrow();

        // Java converts the baseline work with `TimeUnit.HOURS` as the default unit.
        assert_eq!(
            resource.get_baseline_work(),
            Some(Duration::get_instance(120.0, TimeUnit::Hours))
        );
        assert_eq!(resource.get_baseline_cost(), Some(20.005));

        assert_eq!(
            resource.get_baseline_work_at(1),
            Some(Duration::get_instance(40.0, TimeUnit::Hours))
        );
        assert_eq!(resource.get_baseline_cost_at(1), Some(1.0));
    }

    /// An `ExtendedAttribute` is written through `DatatypeConverter.parseCustomField`, i.e. into
    /// the custom field named by `FieldID` (here resource Text1, MPP field ID 205520904).
    #[test]
    fn writes_a_resource_extended_attribute() {
        let (_file, resource) = read_resource(
            r#"<Resource>
                <UID>1</UID>
                <ExtendedAttribute>
                    <FieldID>205520904</FieldID>
                    <Value>Custom text</Value>
                </ExtendedAttribute>
            </Resource>"#,
        );

        assert_eq!(
            resource
                .borrow()
                .get(FieldKey::Resource(ResourceField::Text1)),
            Some(FieldValue::String("Custom text".to_owned()))
        );
    }

    /// An `OutlineCode` element is resolved through the custom field's lookup table: the value ID
    /// selects a `CustomFieldValueItem` whose value is written to the field.
    ///
    /// The lookup table itself is read by the custom field section (`readOutlineCodeDefinitions` /
    /// `readExtendedAttributeDefinitions`), so this test exercises this section together with that
    /// one.
    #[test]
    fn resolves_a_resource_outline_code_through_the_lookup_table() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Project xmlns="http://schemas.microsoft.com/project">
    <DurationFormat>7</DurationFormat>
    <MinutesPerDay>480</MinutesPerDay>
    <OutlineCodes>
        <OutlineCode>
            <Guid>D8C7E507-A694-44B6-A2D2-64B892118ADC</Guid>
            <Enterprise>0</Enterprise>
            <ResourceSubstitutionEnabled>0</ResourceSubstitutionEnabled>
            <LeafOnly>0</LeafOnly>
            <AllLevelsRequired>0</AllLevelsRequired>
            <OnlyTableValuesAllowed>0</OnlyTableValuesAllowed>
            <Values>
                <Value>
                    <ValueID>1</ValueID>
                    <ParentValueID>0</ParentValueID>
                    <Type>21</Type>
                    <Value>AAA</Value>
                </Value>
            </Values>
        </OutlineCode>
    </OutlineCodes>
    <ExtendedAttributes>
        <ExtendedAttribute>
            <FieldID>205521174</FieldID>
            <FieldName>Outline Code1</FieldName>
            <Ltuid>D8C7E507-A694-44B6-A2D2-64B892118ADC</Ltuid>
        </ExtendedAttribute>
    </ExtendedAttributes>
    <Resources>
        <Resource>
            <UID>1</UID>
            <Name>Resource One</Name>
            <OutlineCode>
                <FieldID>205521174</FieldID>
                <ValueID>1</ValueID>
            </OutlineCode>
        </Resource>
    </Resources>
</Project>"#;

        let mut reader = MSPDIReader::new();
        let file = reader
            .read(&mut Cursor::new(xml.as_bytes()))
            .expect("the document should be read");

        let resources: Vec<Rc<RefCell<Resource>>> =
            file.borrow().get_resources().iter().cloned().collect();
        assert_eq!(resources.len(), 1);

        assert_eq!(
            resources[0]
                .borrow()
                .get(FieldKey::Resource(ResourceField::OutlineCode1)),
            Some(FieldValue::String("AAA".to_owned()))
        );
    }

    /// An `OutlineCode` element without a `FieldID` is skipped, and so is one whose `FieldID` does
    /// not map to a field.
    #[test]
    fn ignores_an_outline_code_without_a_usable_field_id() {
        let (_file, resource) = read_resource(
            r#"<Resource>
                <UID>1</UID>
                <OutlineCode>
                    <ValueID>1</ValueID>
                </OutlineCode>
                <OutlineCode>
                    <FieldID>1</FieldID>
                    <ValueID>1</ValueID>
                </OutlineCode>
            </Resource>"#,
        );

        // The first element has no FieldID at all (Java's null check); the second maps to an
        // unknown field (field ID 1 has no prefix), so nothing is written.
        assert_eq!(
            resource
                .borrow()
                .get(FieldKey::Resource(ResourceField::OutlineCode1)),
            None
        );
    }
}
