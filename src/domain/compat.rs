use serde::{Deserialize, Deserializer, de::DeserializeOwned};
use serde_json::Value;

pub fn known<'de, D: Deserializer<'de>, T: DeserializeOwned>(d: D) -> Result<Vec<T>, D::Error> {
    Ok(Vec::<Value>::deserialize(d)?.into_iter().filter_map(|v| serde_json::from_value(v).ok()).collect())
}

pub fn some_known<'de, D: Deserializer<'de>, T: DeserializeOwned>(d: D) -> Result<Option<Vec<T>>, D::Error> {
    Ok(Option::<Vec<Value>>::deserialize(d)?.map(|v| v.into_iter().filter_map(|v| serde_json::from_value(v).ok()).collect()))
}

pub fn or_default<'de, D: Deserializer<'de>, T: DeserializeOwned + Default>(d: D) -> Result<T, D::Error> {
    Ok(serde_json::from_value(Value::deserialize(d)?).unwrap_or_default())
}

pub fn int<'de, D: Deserializer<'de>>(d: D) -> Result<i32, D::Error> {
    Ok(Value::deserialize(d)?.as_f64().map_or(0, |n| n.round() as i32))
}

pub fn index<'de, D: Deserializer<'de>>(d: D) -> Result<usize, D::Error> {
    Ok(Value::deserialize(d)?.as_f64().map_or(0, |n| n.round() as usize))
}

pub fn text<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Ok(match Value::deserialize(d)? {
        Value::String(s) => s,
        _ => String::new(),
    })
}

#[cfg(test)]
mod tests {
    use crate::domain::{
        Config, Fence, Kind, Sort,
        layouts::{Layout, SNAPSHOTS, Snapshot},
        screens::{Adapt, KEPT},
        system::Sys,
    };

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

    #[test]
    fn unknown_custom_fields_are_dropped() {
        let json = r#"{"fences":[{"id":1,"custom":["Spacing","Mica","Hologram"],"tabs":[{"id":1}]},{"id":2,"tabs":[{"id":2}]}]}"#;
        let c: Config = serde_json::from_str(json).unwrap();
        assert_eq!(c.fences[0].custom, Some(vec![crate::domain::inherit::Field::Spacing]));
        assert_eq!(c.fences[1].custom, None);
    }

    #[test]
    fn damaged_layouts_are_bounded_and_capped() {
        let json = r#"{"fences":[{"id":1,"tabs":[{"id":1}]}],"screens":[{"full":[0,0,1920,1080],"work":[0,0,1920,1040]}],
            "snapshots":[{"name":null,"layout":{"screens":[{"full":[0,0,1920,1080],"work":[0,0,1920,1040]}],
            "fences":[[1,[0,0,2000000000,500]],[2,"bad"]]}},{"name":"x","layout":5}],
            "layouts":[{"screens":[{"full":[0,0,99,99],"work":[0,0,99,99]}],"fences":[[1,[-2000000000,0,10,10]]]}]}"#;
        let mut c = Config::parse(json.as_bytes()).unwrap().sanitized();
        assert_eq!(
            (c.snapshots.len(), c.snapshots[0].name.as_str(), c.snapshots[0].layout.fences.clone()),
            (1, "", vec![(1, [0, 0, 10_000, 500])])
        );
        assert_eq!(c.layouts[0].fences, vec![(1, [-20_000, 0, Fence::MIN.0, Fence::MIN.1])]);
        assert!(c.restore(0) && (c.fences[0].x, c.fences[0].w) == (0, 10_000));
        c.snapshots[0].layout.fences = vec![(1, [i32::MAX, i32::MAX, i32::MAX, i32::MAX])];
        c.screens[0].full = [0, 0, 3840, 2160];
        assert!(c.restore(0) && c.fences[0].x <= 20_000 && c.fences[0].w <= 10_000);
        c.layouts = vec![Layout::default(); 30];
        c.snapshots = (0..30).map(|i| Snapshot { name: format!("S{i}"), ..Snapshot::default() }).collect();
        let c = c.sanitized();
        assert_eq!((c.layouts.len(), c.snapshots.len(), c.snapshots[0].name.as_str()), (KEPT, SNAPSHOTS, "S20"));
        assert_eq!(c.clone().sanitized(), c);
    }
}
