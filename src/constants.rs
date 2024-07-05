struct ConstantVisitor(Vec<(String, String)>);

impl<'de> serde::de::Visitor<'de> for ConstantVisitor {
    type Value = Vec<(String, String)>;
    fn expecting(&self, _formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        todo!("return nice descriptive error")
    }

    fn visit_map<A>(mut self, mut map: A) -> Result<Self::Value, A::Error>
where
        A: serde::de::MapAccess<'de>,
    {
        while let Some((name, value)) = map.next_entry()? {
            self.0.push((name, value));
        }
        Ok(self.0)
    }
}

pub fn parse<'de, D>(des: D) -> Result<Vec<(String, String)>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    des.deserialize_map(ConstantVisitor(vec![]))
}
