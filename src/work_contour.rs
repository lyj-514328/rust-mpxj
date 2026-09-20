//! 对应 Java: `org/mpxj/WorkContour.java`

use std::fmt::{self, Display, Formatter};

/// 工时分布（work contour）值。
///
/// 对应 Java: `public final class WorkContour implements ProjectEntityWithUniqueID`
///
/// 移植说明：
/// - Java 侧实现 `ProjectEntityWithUniqueID` 接口；该 trait 已落地，本仓的 `impl` 写在
///   [`crate::work_contour_container`]（早期并行开发期为避免跨文件冲突而放在容器文件内，
///   属待归位项，见 `TECH_DEBT.md` 六），本类型自身仍保留同名的
///   [`WorkContour::get_unique_id`] 方法。
/// - Java 的 `double... curveValues` 可变参数映射为 `Option<Vec<f64>>`：`null` 表示曲线
///   已被手工编辑（`CONTOURED`），与 Java 的 `null` 语义一致。
/// - Java 的 `public static final` 常量（`FLAT`、`BACK_LOADED` 等）无法在 Rust 中 `const`
///   构造（`Vec<f64>` / `String` 都不是 const 可构造类型），因此改为同名函数返回新实例。
///
/// 未移植的 Java 方法：无。
#[derive(Debug, Clone)]
pub struct WorkContour {
    /// 工时分布名称。
    ///
    /// 对应 Java: `private final String m_name;`
    m_name: Option<String>,
    /// 是否为源应用程序内置的工时分布。
    ///
    /// 对应 Java: `private final boolean m_default;`
    m_default: bool,
    /// 唯一 ID。
    ///
    /// 对应 Java: `private final Integer m_uniqueID;`
    m_unique_id: Option<i32>,
    /// 曲线取值：每 5% 工期一个值（含 0%），共 21 个值，总和为 100%。
    ///
    /// 对应 Java: `private final double[] m_curveValues;`
    m_curve_values: Option<Vec<f64>>,
}

impl WorkContour {
    /// 构造方法。
    ///
    /// 对应 Java: `public WorkContour(Integer uniqueID, String name, boolean isDefault, double... curveValues)`
    ///
    /// `curve_values` 为 `None` 对应 Java 传入 `null`（手工编辑的时间分段数据）。
    pub fn new(
        unique_id: Option<i32>,
        name: Option<String>,
        is_default: bool,
        curve_values: Option<Vec<f64>>,
    ) -> Self {
        Self {
            m_name: name,
            m_default: is_default,
            m_unique_id: unique_id,
            m_curve_values: curve_values,
        }
    }

    /// 取唯一 ID。
    ///
    /// 对应 Java: `@Override public Integer getUniqueID()`
    pub fn get_unique_id(&self) -> Option<i32> {
        self.m_unique_id
    }

    /// 取定义曲线的取值数组。
    ///
    /// 对应 Java: `public double[] getCurveValues()`
    ///
    /// 返回 21 个 `double`，各代表 5% 工期（含 0%），总和为 100%；`CONTOURED` 返回 `None`
    /// （Java 返回 `null`），因为其工时/成本已手工编排，不使用曲线。
    pub fn get_curve_values(&self) -> Option<&[f64]> {
        self.m_curve_values.as_deref()
    }

    /// 取工时分布名称。
    ///
    /// 对应 Java: `public String getName()`
    pub fn get_name(&self) -> Option<&str> {
        self.m_name.as_deref()
    }

    /// 判断时间分段数据是否已被手工编辑。
    ///
    /// 对应 Java: `public boolean isContourManual()`
    pub fn is_contour_manual(&self) -> bool {
        self.m_curve_values.is_none()
    }

    /// 判断该工时分布是否为「平坦」的。
    ///
    /// 对应 Java: `public boolean isContourFlat()`
    ///
    /// Java: `m_curveValues != null && m_curveValues[0] == 0.0 &&
    /// DoubleStream.of(m_curveValues).skip(1).distinct().count() == 1`。
    pub fn is_contour_flat(&self) -> bool {
        let values = match &self.m_curve_values {
            Some(values) => values,
            None => return false,
        };

        if values.first() != Some(&0.0) {
            return false;
        }

        // Java: 跳过首个取值后，剩余取值去重后只剩 1 个（即全部相同）
        let mut tail = values.iter().skip(1);
        match tail.next() {
            Some(first) => tail.all(|value| value == first),
            None => false,
        }
    }

    /// 判断是否为源应用程序内置的工时分布。
    ///
    /// 对应 Java: `public boolean isContourDefault()`
    pub fn is_contour_default(&self) -> bool {
        self.m_default
    }

    /// `FLAT` 工时分布。
    ///
    /// 对应 Java: `public static final WorkContour FLAT`
    pub fn flat() -> Self {
        Self::new(
            Some(1),
            Some("FLAT".to_owned()),
            true,
            Some(vec![
                0.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0,
                5.0, 5.0, 5.0, 5.0, 5.0,
            ]),
        )
    }

    /// `BACK_LOADED` 工时分布。
    ///
    /// 对应 Java: `public static final WorkContour BACK_LOADED`
    pub fn back_loaded() -> Self {
        Self::new(
            Some(2),
            Some("BACK_LOADED".to_owned()),
            true,
            Some(vec![
                0.0, 3.5, 3.5, 3.5, 3.5, 3.5, 3.5, 3.5, 3.5, 3.5, 3.5, 6.5, 6.5, 6.5, 6.5, 6.5,
                6.5, 6.5, 6.5, 6.5, 6.5,
            ]),
        )
    }

    /// `FRONT_LOADED` 工时分布。
    ///
    /// 对应 Java: `public static final WorkContour FRONT_LOADED`
    pub fn front_loaded() -> Self {
        Self::new(
            Some(3),
            Some("FRONT_LOADED".to_owned()),
            true,
            Some(vec![
                0.0, 6.5, 6.5, 6.5, 6.5, 6.5, 6.5, 6.5, 6.5, 6.5, 6.5, 3.5, 3.5, 3.5, 3.5, 3.5,
                3.5, 3.5, 3.5, 3.5, 3.5,
            ]),
        )
    }

    /// `DOUBLE_PEAK` 工时分布。
    ///
    /// 对应 Java: `public static final WorkContour DOUBLE_PEAK`
    pub fn double_peak() -> Self {
        Self::new(
            Some(4),
            Some("DOUBLE_PEAK".to_owned()),
            true,
            Some(vec![
                0.0, 1.3, 2.5, 3.8, 5.1, 7.6, 10.1, 7.6, 5.1, 3.8, 2.5, 2.5, 2.5, 3.8, 5.1, 7.6,
                10.1, 7.6, 5.1, 3.8, 2.5,
            ]),
        )
    }

    /// `EARLY_PEAK` 工时分布。
    ///
    /// 对应 Java: `public static final WorkContour EARLY_PEAK`
    pub fn early_peak() -> Self {
        Self::new(
            Some(5),
            Some("EARLY_PEAK".to_owned()),
            true,
            Some(vec![
                0.0, 1.2, 2.5, 3.8, 5.0, 7.5, 10.1, 10.1, 10.1, 8.8, 7.5, 6.3, 5.0, 5.0, 5.0, 3.8,
                2.5, 2.0, 1.5, 1.3, 1.0,
            ]),
        )
    }

    /// `LATE_PEAK` 工时分布。
    ///
    /// 对应 Java: `public static final WorkContour LATE_PEAK`
    pub fn late_peak() -> Self {
        Self::new(
            Some(6),
            Some("LATE_PEAK".to_owned()),
            true,
            Some(vec![
                0.0, 1.0, 1.3, 1.5, 2.0, 2.5, 3.8, 5.0, 5.0, 5.0, 6.3, 7.5, 8.8, 10.1, 10.1, 10.1,
                7.5, 5.0, 3.8, 2.5, 1.2,
            ]),
        )
    }

    /// `BELL` 工时分布。
    ///
    /// 对应 Java: `public static final WorkContour BELL`
    pub fn bell() -> Self {
        Self::new(
            Some(7),
            Some("BELL".to_owned()),
            true,
            Some(vec![
                0.0, 0.5, 0.5, 1.5, 1.5, 4.0, 4.0, 7.5, 7.5, 11.5, 11.5, 11.5, 11.5, 7.5, 7.5, 4.0,
                4.0, 1.5, 1.5, 0.5, 0.5,
            ]),
        )
    }

    /// `TURTLE` 工时分布。
    ///
    /// 对应 Java: `public static final WorkContour TURTLE`
    pub fn turtle() -> Self {
        Self::new(
            Some(8),
            Some("TURTLE".to_owned()),
            true,
            Some(vec![
                0.0, 1.0, 1.0, 3.5, 3.5, 5.5, 5.5, 7.5, 7.5, 7.5, 7.5, 7.5, 7.5, 7.5, 7.5, 5.5,
                5.5, 3.5, 3.5, 1.0, 1.0,
            ]),
        )
    }

    /// `CONTOURED` 工时分布（无曲线取值，对应 Java 传入 `null`）。
    ///
    /// 对应 Java: `public static final WorkContour CONTOURED`
    pub fn contoured() -> Self {
        Self::new(Some(9), Some("CONTOURED".to_owned()), false, None)
    }
}

impl Display for WorkContour {
    /// 对应 Java: `@Override public String toString()`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.m_name.as_deref().unwrap_or("null"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accessors_return_constructor_values() {
        let value = WorkContour::new(
            Some(42),
            Some("CUSTOM".to_owned()),
            false,
            Some(vec![0.0, 100.0]),
        );

        assert_eq!(value.get_unique_id(), Some(42));
        assert_eq!(value.get_name(), Some("CUSTOM"));
        assert_eq!(value.get_curve_values(), Some(&[0.0, 100.0][..]));
        assert!(!value.is_contour_default());
        assert!(!value.is_contour_manual());
        assert!(value.is_contour_flat());
    }

    #[test]
    fn flat_contour_matches_java_definition() {
        let value = WorkContour::flat();

        assert_eq!(value.get_unique_id(), Some(1));
        assert_eq!(value.get_name(), Some("FLAT"));
        assert!(value.is_contour_default());
        assert!(!value.is_contour_manual());
        assert!(value.is_contour_flat());

        let curve = value.get_curve_values().expect("FLAT 有曲线取值");
        assert_eq!(curve.len(), 21);
        assert_eq!(curve.first(), Some(&0.0));
    }

    #[test]
    fn non_flat_curve_is_detected() {
        let value = WorkContour::back_loaded();

        assert_eq!(value.get_unique_id(), Some(2));
        assert_eq!(value.get_name(), Some("BACK_LOADED"));
        assert!(value.is_contour_default());
        assert!(!value.is_contour_flat());
        assert_eq!(value.get_curve_values().map(|curve| curve.len()), Some(21));
    }

    #[test]
    fn contoured_contour_is_manual() {
        let value = WorkContour::contoured();

        assert_eq!(value.get_unique_id(), Some(9));
        assert_eq!(value.get_name(), Some("CONTOURED"));
        assert!(!value.is_contour_default());
        assert!(value.is_contour_manual());
        assert!(!value.is_contour_flat());
        assert!(value.get_curve_values().is_none());
    }

    #[test]
    fn built_in_contours_are_all_defined() {
        let contours = [
            WorkContour::flat(),
            WorkContour::back_loaded(),
            WorkContour::front_loaded(),
            WorkContour::double_peak(),
            WorkContour::early_peak(),
            WorkContour::late_peak(),
            WorkContour::bell(),
            WorkContour::turtle(),
            WorkContour::contoured(),
        ];

        for (index, contour) in contours.iter().enumerate() {
            assert_eq!(contour.get_unique_id(), Some(index as i32 + 1));
            assert!(contour.get_name().is_some());
        }

        // 除 CONTOURED 外都带 21 个曲线取值
        for contour in &contours[..8] {
            assert_eq!(contour.get_curve_values().map(|curve| curve.len()), Some(21));
        }
    }

    #[test]
    fn display_returns_name() {
        assert_eq!(WorkContour::bell().to_string(), "BELL");
        assert_eq!(
            WorkContour::new(None, None, false, None).to_string(),
            "null"
        );
    }
}
