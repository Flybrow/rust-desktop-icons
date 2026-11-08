use serde::{Deserialize, Deserializer, de::DeserializeOwned};
use serde_json::Value;

pub fn known<'de, D: Deserializer<'de>, T: DeserializeOwned>(d: D) -> Result<Vec<T>, D::Error> {
    Ok(Vec::<Value>::deserialize(d)?.into_iter().filter_map(|v| serde_json::from_value(v).ok()).collect())
}

pub fn or_default<'de, D: Deserializer<'de>, T: DeserializeOwned + Default>(d: D) -> Result<T, D::Error> {
    Ok(serde_json::from_value(Value::deserialize(d)?).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use crate::domain::{Config, Kind, Sort, screens::Adapt, system::Sys};

    #[test]
    fn newer_values_do_not_break_the_config() {
        let json = r#"{"screen_adapt":"Teleport","desk_icons":[["Printer",1],["ThisPc",null]],
            "fences":[{"id":1,"tabs":[{"id":1,"sort":"Color","kinds":["Images","Spreadsheets"],"system":["Phone","RecycleBin"]}]}]}"#;
        let c: Config = serde_json::from_str(json).unwrap();
        let t = &c.fences[0].tabs[0];
        assert_eq!((c.screen_adapt, t.sort), (Adapt::default(), Sort::default()));
        assert_eq!((t.kinds.clone(), t.system.clone()), (vec![Kind::Images], vec![Sys::RecycleBin]));
        assert_eq!(c.desk_icons, vec![(Sys::ThisPc, None)]);
    }
}
