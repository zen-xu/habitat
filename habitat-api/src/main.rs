use kube::CustomResourceExt;

fn main() {
    println!("{}", serde_yaml::to_string(&habitat_api::Job::crd()).unwrap());
}
