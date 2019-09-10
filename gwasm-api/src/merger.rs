

pub trait Merger<In, Out> {
    fn merge(self, args_vec: &Vec<String>, tasks: Vec<(In, Out)>);
}


