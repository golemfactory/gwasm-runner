
pub trait Merger<In, Out> {

    fn merge(self, tasks : Vec<(In, Out)>);

}
