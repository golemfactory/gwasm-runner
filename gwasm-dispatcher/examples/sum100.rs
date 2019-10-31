use gwasm_dispatcher::*;

fn main() {
    dispatcher::run(
        move |_: &mut dyn SplitContext| {
            const NUM_SUBTASKS: usize = 10;
            let arr: Vec<u64> = (1..=100).collect();
            arr.chunks(NUM_SUBTASKS)
                .map(|x| (x.to_vec(),))
                .collect::<Vec<_>>()
        },
        |task: Vec<u64>| (task.into_iter().sum(),),
        |_: &Vec<String>, results: Vec<(_, _)>| {
            let given: u64 = results.iter().map(|(_, (result,))| result).sum();
            let expected: u64 = (1..=100).sum();
            assert_eq!(expected, given, "sums should be equal")
        },
    )
    .unwrap()
}
