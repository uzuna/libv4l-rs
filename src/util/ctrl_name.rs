/// Convert a string to a control name.
///
/// `v4l2-ctl`で指定するときはsnake_caseを使う
/// v4l2のControlのNameで同じものを得るための変換方法を定義している
pub trait ToCtrlName: AsRef<str> {
    fn to_ctrl_name(&self) -> String;
}

impl<T> ToCtrlName for T
where
    T: AsRef<str>,
{
    fn to_ctrl_name(&self) -> String {
        let text = self.as_ref();

        text.replace(" ", "_").replace(",", "").to_lowercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_ctrl_name() {
        let td = vec![
            ("Exposure Time, Absolute", "exposure_time_absolute"),
            ("Sensor Mode", "sensor_mode"),
            ("White Balance Temperature", "white_balance_temperature"),
            ("Height Align", "height_align"),
        ];

        for (input, expected) in td {
            assert_eq!(expected, input.to_ctrl_name().as_str());
        }
    }
}
