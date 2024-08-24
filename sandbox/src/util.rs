use serde_json::Value;
use std::collections::BTreeMap;

fn count<I, T, F>(iter: I, condition: F) -> usize
where
    I: IntoIterator<Item = T>,
    F: Fn(&T) -> bool,
{
    iter.into_iter().filter(|item| condition(item)).count()
}

fn get_json_type(value: &Value) -> &'static str {
    match value {
        Value::Null => "Null",
        Value::Bool(_) => "Bool",
        Value::Number(_) => "Number",
        Value::String(_) => "String",
        Value::Array(_) => "Array",
        Value::Object(_) => "Object",
    }
}

fn _get_fields(values: &[Value]) -> BTreeMap<String, String> {
    values
        .iter()
        .filter_map(|v| v.as_object())
        .flat_map(|obj| {
            obj.iter()
                .map(|(k, v)| (k.to_string(), get_json_type(v).to_string()))
        })
        .collect()
}

pub(crate) fn get_fields(values: &[Value]) -> BTreeMap<String, String> {
    let mut fields = _get_fields(values);
    let total_objects = values.iter().filter(|v| v.is_object()).count();

    for (field, type_) in fields.iter_mut() {
        let field_count = values
            .iter()
            .filter_map(|v| v.as_object())
            .filter(|obj| obj.contains_key(field))
            .count();

        if field_count < total_objects {
            *type_ = format!("{} (Optional)", type_);
        }
    }

    fields
}
