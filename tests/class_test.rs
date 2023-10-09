use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::io::Read;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Case {
    args: Vec<String>,
    stdout: String,
    trace: Option<String>,
}

impl Display for Case {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "args: {:?}, stdout: {:?}", self.args, self.stdout)
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct CaseYaml {
    class_name: String,
    cases: Vec<Case>,
}

macro_rules! test_class {
    ($class_name:ident) => {
        let path = yaml_path(stringify!($class_name));
        let mut file = File::open(path.clone()).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        let test_suite: CaseYaml = serde_yaml::from_str(&contents).unwrap();

        let mut env = JitEnv::new(stringify!($class_name));
        env.enable_tracing();
        env.compile(path.to_str().unwrap().replace(".yaml", ".class").as_str());
        env.done_compilation();
        env.dump_llvm_module(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("out.ll")
                .to_str()
                .unwrap(),
        );

        println!(
            ">>>>>>>>>>>>>>>>>> Running test: {}",
            stringify!($class_name)
        );
        for case in test_suite.cases {
            print!("Case: {} \n", case);
            let mut isolate = env.new_isolate(StdoutOption::VecOutputStream);
            env.call(&mut isolate, &case.args);
            let buf = isolate.stdout_buffer();
            let s = String::from_utf8(buf.to_vec()).unwrap();
            let traced = isolate.tracer().buffer();
            println!("Tracing:\n{}", String::from_utf8(traced.to_vec()).unwrap());
            assert_eq!(s, case.stdout, "\nleft:\n{}\nright:\n{}", s, case.stdout);
            println!("\tPassed");
        }
        println!("<<<<<<<<<<<<<< {} Passed\n\n", stringify!($class_name));
    };
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fs::File;
    use std::path::PathBuf;
    use yajvm::{JitEnv, StdoutOption};

    fn yaml_path(class_name: &str) -> PathBuf {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests");
        path.push("cases");
        path.push(class_name.to_string() + ".yaml");
        path
    }

    #[test]
    fn test_hello_world() {
        test_class!(HelloWorld);
    }

    #[test]
    fn test_print_first_arg() {
        test_class!(PrintFirstArg);
    }

    #[test]
    fn test_print_args() {
        test_class!(PrintArgs);
    }

    #[test]
    fn test_print_arg_len() {
        test_class!(PrintArgLen);
    }

    #[test]
    fn test_in_class_call() {
        test_class!(InClassCall);
    }

    #[test]
    fn test_integers() {
        test_class!(Integers);
    }

    #[test]
    fn test_integers2() {
        test_class!(Integers2);
    }

    #[test]
    fn test_numerics() {
        test_class!(Numerics);
    }

    #[test]
    fn test_static_variables() {
        test_class!(StaticVariables);
    }

    #[test]
    fn test_nested_method_calls() {
        test_class!(NestedMethodCalls);
    }

    #[test]
    fn test_mutual_recursion() {
        test_class!(MutualRecursion);
    }

    #[test]
    fn test_factorial_recursion() {
        test_class!(FactorialRecursion);
    }

    #[test]
    fn test_comparisons() {
        test_class!(Comparisons);
    }

    #[test]
    fn test_string_const_return() {
        test_class!(StringConstReturn);
    }

    #[test]
    fn test_string_const_arg() {
        test_class!(StringConstArg);
    }

    #[test]
    fn test_fcmp_nan() {
        test_class!(FcmpNan);
    }

    #[test]
    fn test_all_basic_types_params() {
        test_class!(AllBasicTypesParams);
    }

    #[test]
    fn test_basic_type_array() {
        test_class!(BasicTypeArray);
    }

    #[test]
    fn test_inc_dec() {
        test_class!(IncDec);
    }
}
