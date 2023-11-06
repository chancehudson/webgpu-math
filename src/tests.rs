use super::*;

#[test]
fn should_multiply() {
    let input = [21414, 78788, 29141849, 0];
    let calc = async move {
        if let Some(produced) = execute_gpu(&input).await {
            assert_eq!(produced[3], (input[0] * input[1]) % input[2]);
        }
    };
    pollster::block_on(calc);
}
