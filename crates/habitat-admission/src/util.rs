use habitat_api::Job;
use kube::core::DynamicObject;
use lazy_static::lazy_static;
use regex::Regex;

pub fn try_cast_dynamic_obj_into_job(obj: &DynamicObject) -> Result<Job, String> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r" at line \d+ column \d+").unwrap();
    }

    let obj_json = serde_json::to_string(obj).unwrap();
    serde_json::from_str(&obj_json).map_err(|e| RE.replace(&e.to_string(), "").to_string())
}
