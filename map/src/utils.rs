use glam::DVec3;

use crate::{Attributes, Entity, Map};

impl Map {
    pub fn get_entities_by_classname<'a>(
        &'a self,
        classname: &'a str,
    ) -> impl Iterator<Item = &'a Entity> + 'a {
        self.entities.iter().filter(move |x| {
            x.attributes
                .get("classname".into())
                .is_some_and(|classname_curr| classname_curr == classname)
        })
    }

    pub fn get_entities_by_classname_mut<'a>(
        &'a mut self,
        classname: &'a str,
    ) -> impl Iterator<Item = &'a mut Entity> + 'a {
        self.entities.iter_mut().filter(move |x| {
            x.attributes
                .get("classname".into())
                .is_some_and(|classname_curr| classname_curr == classname)
        })
    }

    pub fn get_entity_by_class_name_first(&self, classname: &str) -> Option<usize> {
        self.entities.iter().position(|x| {
            x.attributes
                .get("classname".into())
                .is_some_and(|classname_curr| classname_curr == classname)
        })
    }

    pub fn get_entities_by_classname_all(&self, classname: &str) -> Vec<usize> {
        self.entities
            .iter()
            .enumerate()
            .filter_map(|(idx, x)| {
                if x.attributes
                    .get("classname".into())
                    .is_some_and(|classname_curr| classname_curr == classname)
                {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn get_entity_by_targetname(&self, targetname: &str) -> Option<usize> {
        self.entities.iter().position(|x| {
            x.attributes
                .get("targetname".into())
                .is_some_and(|targetname_curr| targetname_curr == targetname)
        })
    }

    pub fn get_entity_by_classname_and_targetname(
        &self,
        classname: &str,
        targetname: &str,
    ) -> Option<usize> {
        self.entities.iter().position(|x| {
            x.attributes
                .get("classname".into())
                .is_some_and(|classname_curr| classname_curr == classname)
                && x.attributes
                    .get("targetname".into())
                    .is_some_and(|targetname_curr| targetname_curr == targetname)
        })
    }

    pub fn insert_new_point_entity(&mut self, attributes: Attributes) {
        let new_entity = Entity {
            attributes,
            brushes: None,
        };

        self.entities.push(new_entity);
    }
}

impl Entity {
    pub fn origin(&self) -> Option<DVec3> {
        self.attributes.get("origin").and_then(|x| parse_triplet(x))
    }

    pub fn angles(&self) -> Option<DVec3> {
        self.attributes.get("angles").and_then(|x| parse_triplet(x))
    }

    pub fn scale(&self) -> Option<f64> {
        self.attributes
            .get("scale")
            .and_then(|x| x.parse::<f64>().ok())
    }

    pub fn sequence(&self) -> Option<u32> {
        self.attributes
            .get("sequence")
            .and_then(|x| x.parse::<u32>().ok())
    }

    pub fn targetname(&self) -> Option<&String> {
        self.attributes.get("targetname")
    }

    pub fn target(&self) -> Option<&String> {
        self.attributes.get("target")
    }

    pub fn spawnflags(&self) -> Option<u32> {
        self.attributes
            .get("spawnflags")
            .and_then(|x| x.parse::<u32>().ok())
    }

    pub fn model(&self) -> Option<&String> {
        self.attributes.get("model")
    }

    pub fn classname(&self) -> Option<&String> {
        self.attributes.get("classname")
    }
}

fn parse_triplet(i: &str) -> Option<DVec3> {
    let res = i
        .split_ascii_whitespace()
        .filter_map(|i| i.parse::<f64>().ok())
        .collect::<Vec<f64>>();

    if res.len() < 3 {
        return None;
    }

    Some([res[0], res[1], res[2]].into())
}
