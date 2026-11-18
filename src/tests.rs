#[cfg(test)]
// use cargo test -- --test-threads=1
mod tests {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };

    use crate::{compile, compile_and_link};

    #[test]
    fn empty() {
        compile_and_link(
            "out.o",
            "out",
            r#"

            fn main() -> i32 {
                return 0;
            }
        "#,
        )
        .unwrap();

        let status = Command::new("./out").status().unwrap().code().unwrap();
        assert_eq!(status, 0);
    }

    #[test]
    fn string_lit() {
        compile_and_link(
            "out.o",
            "out",
            r#"
            fn printf(format: *char, ...);
            fn main() -> i32 {
                printf("Hello world\r\n\t\"\'\\");
                return 0;
            }
        "#,
        )
        .unwrap();

        let status = Command::new("./out").output().unwrap();
        assert_eq!(
            str::from_utf8(&status.stdout).unwrap(),
            "Hello world\r\n\t\"\'\\"
        );
    }

    #[test]
    fn int() {
        compile_and_link(
            "out.o",
            "out",
            r#"
            fn main() -> i32 {
                let a = 15;
                let b = 8 + a;
                return a * 3 - b;
            }
        "#,
        )
        .unwrap();

        let status = Command::new("./out").status().unwrap().code().unwrap();
        assert_eq!(status, 22);
    }

    #[test]
    fn invalid_parse() {
        assert!(
            compile(
                "out.o",
                r#"
            fn main() -> i32 {
                let b
                return 0;
            }
        "#,
            )
            .is_err()
        );
    }

    #[test]
    fn invalid_var() {
        assert!(
            compile(
                "out.o",
                r#"
            fn main() -> i32 {
                let b = 5;
                return a * 3 - b;
            }
        "#,
            )
            .is_err()
        );
    }

    #[test]
    fn invalid_type() {
        assert!(
            compile(
                "out.o",
                r#"
            fn main() -> i32 {
                let a = 3.14;
                return a;
            }
        "#,
            )
            .is_err()
        );
    }

    #[test]
    fn invalid_fn() {
        assert!(
            compile(
                "out.o",
                r#"
            fn main() -> i32 {
                hello();
                return 0;
            }
        "#,
            )
            .is_err()
        );
    }

    #[test]
    fn invalid_fn_args() {
        assert!(
            compile(
                "out.o",
                r#"
            fn hello(a: i32);
            fn main() -> i32 {
                hello();
                return 0;
            }
        "#,
            )
            .is_err()
        );
    }

    #[test]
    fn interop() {
        compile(
            "out.o",
            r#"
            struct Small {
                a: i32,
                b: i32
            }

            struct Large {
                a: i32,
                b: f32,
                c: i64,
                d: i64
            }
            fn process(s: Small, a: i32) -> Large;
            fn printf(format: *char, ...);
            fn main() -> i32 {
                let l = process(Small {a: 1, b: 2}, 3);
                printf("%d, %.2f, %d, %d", l.a, l.b, l.c, l.d);
                return 0;
            }
        "#,
        )
        .unwrap();

        let mut gcc = Command::new("gcc")
            .args(["-o", "/tmp/c.o", "-xc", "-c", "-"])
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        gcc.stdin
            .take()
            .unwrap()
            .write_all(
                r#"
                #include <stdint.h>
                struct Small {
                  int32_t a;
                  int32_t b;
                };

                struct Large {
                  int32_t a;
                  float b;
                  int64_t c;
                  int64_t d;
                };

                struct Large process(struct Small s, int32_t a) {
                  return (struct Large){.a = s.a, .b = 3.14, .c = s.b, .d = a};
                }
                "#
                .as_bytes(),
            )
            .unwrap();
        gcc.wait().unwrap();

        Command::new("gcc")
            .args(["out.o", "/tmp/c.o", "-o", "out"])
            .status()
            .expect("Couldn't link the executable.");

        let status = Command::new("./out").output().unwrap();
        assert_eq!(str::from_utf8(&status.stdout).unwrap(), "1, 3.14, 2, 3",);
    }

    #[test]
    fn interop2() {
        compile(
            "out.o",
            r#"
            struct Small {
                a: i32,
                b: i32
            }

            struct Large {
                a: i32,
                b: f32,
                c: i64,
                d: i64
            }
            fn process(s: Small, a: i32) -> Large {
                return Large {
                    a: s.a,
                    b: 3.14,
                    c: s.b as i64,
                    d: a as i64
                };
            }
        "#,
        )
        .unwrap();

        let mut gcc = Command::new("gcc")
            .args(["-o", "/tmp/c.o", "-xc", "-c", "-"])
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        gcc.stdin
            .take()
            .unwrap()
            .write_all(
                r#"
                #include <stdint.h>
                #include <stdio.h>
                struct Small {
                  int32_t a;
                  int32_t b;
                };

                struct Large {
                  int32_t a;
                  float b;
                  int64_t c;
                  int64_t d;
                };

                struct Large process(struct Small s, int32_t a);
                int main() {
                    struct Large l = process((struct Small) {.a = 1, .b = 2}, 3);
                    printf("%d, %.2f, %d, %d", l.a, l.b, l.c, l.d);
                    return 0;
                }
                "#
                .as_bytes(),
            )
            .unwrap();
        gcc.wait().unwrap();

        Command::new("gcc")
            .args(["out.o", "/tmp/c.o", "-o", "out"])
            .status()
            .expect("Couldn't link the executable.");

        let status = Command::new("./out").output().unwrap();
        assert_eq!(str::from_utf8(&status.stdout).unwrap(), "1, 3.14, 2, 3",);
    }

    #[test]
    fn arrays() {
        compile_and_link(
            "out.o",
            "out",
            r#"
                fn printf(format: *char, ...);
                fn strcpy(dst: *char, src: *char) -> *char;

                fn func(a: [i32; 3]) -> [i32; 3] {
                    a[0] = 0;
                    printf("%d, %d, %d\n", a[0], a[1], a[2]);
                    return a;
                }

                fn main() -> i32 {
                    let a: [i32; 3] = [1, 2, 3];
                    let b = func(a);
                    printf("%d, %d, %d\n", a[0], a[1], a[2]);
                    printf("%d, %d, %d\n", b[0], b[1], b[2]);

                    let str: [char; 16];
                    strcpy(&str as *char, "Hello!");
                    printf("%s\n", &str);
                    return 0;
                }
        "#,
        )
        .unwrap();

        let status = Command::new("./out").output().unwrap();
        assert_eq!(
            str::from_utf8(&status.stdout).unwrap(),
            "0, 2, 3\n1, 2, 3\n0, 2, 3\nHello!\n"
        );
    }
}
