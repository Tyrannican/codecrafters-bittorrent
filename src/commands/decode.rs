use anyhow::Result;

pub(crate) fn invoke(value: &str) -> Result<(serde_json::Value, &str)> {
    match value.chars().next() {
        Some('0'..='9') => {
            if let Some((size, rest)) = value.split_once(':') {
                if let Ok(size) = size.parse::<usize>() {
                    return Ok((rest[..size].to_string().into(), &rest[size..]));
                }
            }
        }
        Some('i') => {
            let value = &value[1..];
            if let Some((val, rest)) = value.split_once('e').and_then(|(digits, rest)| {
                let n = digits.parse::<i64>().ok()?;
                Some((n, rest))
            }) {
                return Ok((val.into(), rest));
            }
        }
        Some('l') => {
            let mut values = Vec::new();
            let mut rest = &value[1..];
            while !rest.is_empty() && !rest.starts_with('e') {
                let (v, remainder) = invoke(rest)?;
                values.push(v);
                rest = remainder;
            }

            return Ok((values.into(), &rest[1..]));
        }
        Some('d') => {
            let mut dict = serde_json::Map::new();
            let mut rest = &value[1..];
            while !rest.is_empty() && !rest.starts_with('e') {
                let (key, remainder) = invoke(rest)?;

                let key = match key {
                    serde_json::Value::String(key) => key,
                    key => {
                        panic!("dict strings must be keys, not {key:?}");
                    }
                };

                let (v, remainder) = invoke(remainder)?;
                dict.insert(key, v);
                rest = remainder;
            }

            return Ok((dict.into(), &rest[1..]));
        }
        _ => {}
    }

    panic!("unrecognized value: {value}");
}
