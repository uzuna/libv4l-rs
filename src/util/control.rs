use std::{collections::BTreeMap, convert::TryFrom};

use super::ctrl_name::ToCtrlName;
use crate::{control::Value as CValue, Control};

/// 変更リクエストを保持する構造体
pub struct Requests {
    requests: Vec<Request>,
}

impl Requests {
    pub fn new(requests: Vec<Request>) -> Self {
        Self { requests }
    }
}

impl TryFrom<&str> for Requests {
    type Error = String;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let requests = value
            .split(',')
            .map(Request::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self::new(requests))
    }
}

/// ユーザーが記述する変更リクエストを保持する構造体
pub struct Request {
    name: String,
    value: Value,
}

impl Request {
    pub fn new(name: &str, value: Value) -> Self {
        Self {
            name: name.to_string(),
            value,
        }
    }
}

impl TryFrom<&str> for Request {
    type Error = String;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut split = value.splitn(2, '=');
        let name = split.next().ok_or(format!("field not found [{}]", value))?;
        let value = split.next().ok_or(format!("value not found [{}]", value))?;
        Ok(Self {
            name: name.to_string(),
            value: Value::from(value),
        })
    }
}

/// 変更リクエストで設定可能な値の種類
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(i64),
    Boolean(bool),
    String(String),
}

impl From<Value> for CValue {
    fn from(val: Value) -> Self {
        match val {
            Value::Integer(i) => CValue::Integer(i),
            Value::Boolean(b) => CValue::Boolean(b),
            Value::String(s) => CValue::String(s),
        }
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        if let Ok(i) = s.parse::<i64>() {
            Value::Integer(i)
        } else if let Ok(b) = s.parse::<bool>() {
            Value::Boolean(b)
        } else {
            Value::String(s.to_string())
        }
    }
}

/// 対象デバイスのControlに対して、設定可能な値やデフォルト値、idなどの情報を保持する構造体
#[derive(Debug)]
pub struct ControlDesc {
    id: u32,
    value: CValue,
    minimum: i64,
    maximum: i64,
}

impl ControlDesc {
    fn check(&self, name: impl Into<String>, value: &Value) -> Option<UnsupportedControlDeatil> {
        match (value, &self.value) {
            (Value::Integer(i), CValue::Integer(_)) => {
                if *i < self.minimum || *i > self.maximum {
                    Some(UnsupportedControlDeatil {
                        name: name.into(),
                        detail: "Out of range".to_string(),
                    })
                } else {
                    None
                }
            }
            (Value::Boolean(_), CValue::Boolean(_)) => None,
            (Value::String(_), CValue::String(_)) => None,
            _ => Some(UnsupportedControlDeatil {
                name: name.into(),
                detail: format!("Type mismatch: {:?} {:?}", value, self.value),
            }),
        }
    }
}

/// 対象デバイスのControlの情報を保持する構造体
///
/// # Example
///
/// ```no_run
/// use v4l::device::Device;
/// use v4l::util::control::{ControlTable, Request, Requests, Value};
///
/// let dev = Device::new(0).unwrap();
/// let ctrlmap = ControlTable::from(dev.query_controls().unwrap().as_slice());
///
/// // リクエストの作成
/// let reqs = Requests::new(vec![
///     Request::new("gain", Value::Integer(0)),
///     Request::new("frame_rate", Value::Integer(100)),
///     Request::new("frame rate", Value::Integer(100)), // 存在しないControl
/// ]);
///
/// // リクエストがサポートされているかチェック
/// let check = ctrlmap.check(&reqs);
/// if !check.is_empty() {
///     println!("{:?}", check);
/// }
///
/// // デフォルト値を設定
/// dev.set_controls(ctrlmap.get_default(&reqs)).unwrap();
///
/// // リクエストの値を設定
/// dev.set_controls(ctrlmap.get_control(&reqs)).unwrap();
/// ```

#[derive(Debug)]
pub struct ControlTable {
    map: BTreeMap<String, ControlDesc>,
}

impl From<&[crate::control::Description]> for ControlTable {
    fn from(controls: &[crate::control::Description]) -> Self {
        use crate::control::{Flags, Type, Value};

        let mut map = BTreeMap::new();

        for control in controls {
            if control.flags & Flags::READ_ONLY == Flags::READ_ONLY {
                continue;
            }

            let value = match control.typ {
                Type::Integer | Type::Integer64 => Value::Integer(control.default),
                Type::Boolean => Value::Boolean(control.default != 0),
                Type::Menu => Value::Integer(control.default),
                // TODO: 他に対応可能な型があれば適宜追加
                _ => continue,
            };
            map.insert(
                control.name.to_ctrl_name(),
                ControlDesc {
                    id: control.id,
                    value,
                    minimum: control.minimum,
                    maximum: control.maximum,
                },
            );
        }

        ControlTable { map }
    }
}

impl ControlTable {
    /// リクエストがサポートされているかチェックする
    pub fn check(&self, reqs: &Requests) -> Vec<UnsupportedControlDeatil> {
        let mut v = vec![];
        for r in reqs.requests.iter() {
            if !self.map.contains_key(r.name.as_str()) {
                v.push(UnsupportedControlDeatil {
                    name: r.name.clone(),
                    detail: "Control not found".to_string(),
                });
            } else {
                let desc = self.map.get(r.name.as_str()).unwrap();
                if let Some(detail) = desc.check(r.name.as_str(), &r.value) {
                    v.push(detail);
                }
            }
        }
        v
    }

    /// リクエストに対応するControlのデフォルト値を取得する
    pub fn get_default(&self, reqs: &Requests) -> Vec<crate::control::Control> {
        let mut v = vec![];
        for r in reqs.requests.iter() {
            if let Some(x) = self.map.get(r.name.as_str()) {
                v.push(Control {
                    id: x.id,
                    value: x.value.clone(),
                });
            }
        }
        v
    }

    /// 設定値に基づいたControlを返す
    pub fn get_control(&self, reqs: &Requests) -> Vec<crate::control::Control> {
        let mut v = vec![];
        for r in reqs.requests.iter() {
            if let Some(x) = self.map.get(r.name.as_str()) {
                v.push(Control {
                    id: x.id,
                    value: r.value.clone().into(),
                });
            }
        }
        v
    }
}

/// 設定不可能なリクエストが来た場合のエラー詳細
#[derive(Debug)]
pub struct UnsupportedControlDeatil {
    pub name: String,
    pub detail: String,
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_unsupported_control_detail() {
        let desc = ControlDesc {
            id: 0,
            value: CValue::Integer(0),
            minimum: 0,
            maximum: 100,
        };

        let td = vec![
            (Value::Integer(-1), false),
            (Value::Integer(0), true),
            (Value::Integer(100), true),
            (Value::Integer(101), false),
            (Value::Boolean(true), false),
        ];
        for (value, expected) in td {
            let detail = desc.check("test", &value);
            assert_eq!(expected, detail.is_none());
        }
    }

    #[test]
    fn test_request_from_str() {
        let td = vec![
            ("gain=0", "gain", Value::Integer(0)),
            ("gain=1", "gain", Value::Integer(1)),
            ("gain=true", "gain", Value::Boolean(true)),
            ("gain=false", "gain", Value::Boolean(false)),
            (
                "white_balance=auto",
                "white_balance",
                Value::String("auto".to_string()),
            ),
        ];

        for (input, name, value) in td {
            let req = Request::try_from(input);
            assert!(req.is_ok());
            let req = req.unwrap();
            assert_eq!(name, req.name);
            assert_eq!(value, req.value);
        }
    }

    #[test]
    fn test_requests_from_str() {
        let td = vec![
            ("gain=0,white_balance=auto", 2),
            ("gain=0,white_balance=auto,exposure=auto", 3),
        ];

        for (input, len) in td {
            let reqs = Requests::try_from(input);
            assert!(reqs.is_ok());
            let reqs = reqs.unwrap();
            assert_eq!(len, reqs.requests.len());
        }
    }
}
