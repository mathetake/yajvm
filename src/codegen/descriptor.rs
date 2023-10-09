// 4.3.2 in https://docs.oracle.com/javase/specs/jvms/se7/html/jvms-4.html#jvms-4.3.2

use std::str::Chars;

#[derive(PartialEq, Debug, Clone)]
pub enum FieldType {
    BaseType(BaseType),
    ObjectTypeJavaLangByte,
    ObjectTypeJavaLangChar,
    ObjectTypeJavaLangDouble,
    ObjectTypeJavaLangFloat,
    ObjectTypeJavaLangInteger,
    ObjectTypeJavaLangLong,
    ObjectTypeJavaLangShort,
    ObjectTypeJavaLangBoolean,
    ObjectType(ObjectType),
    ArrayType(Box<FieldType>),
}

impl std::fmt::Display for FieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            FieldType::BaseType(base_type) => write!(f, "{:?}", base_type),
            FieldType::ObjectType(object_type) => write!(f, "{}", object_type),
            FieldType::ArrayType(array_type) => write!(f, "{}[]", array_type),
            FieldType::ObjectTypeJavaLangByte => write!(f, "java/lang/Byte"),
            FieldType::ObjectTypeJavaLangChar => write!(f, "java/lang/Char"),
            FieldType::ObjectTypeJavaLangDouble => write!(f, "java/lang/Double"),
            FieldType::ObjectTypeJavaLangFloat => write!(f, "java/lang/Float"),
            FieldType::ObjectTypeJavaLangInteger => write!(f, "java/lang/Integer"),
            FieldType::ObjectTypeJavaLangLong => write!(f, "java/lang/Long"),
            FieldType::ObjectTypeJavaLangShort => write!(f, "java/lang/Short"),
            FieldType::ObjectTypeJavaLangBoolean => write!(f, "java/lang/Boolean"),
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum BaseType {
    Byte,
    Char,
    Double,
    Float,
    Int,
    Long,
    Short,
    Boolean,
    Void,
}

impl std::fmt::Display for BaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            BaseType::Byte => write!(f, "byte"),
            BaseType::Char => write!(f, "char"),
            BaseType::Double => write!(f, "double"),
            BaseType::Float => write!(f, "float"),
            BaseType::Int => write!(f, "int"),
            BaseType::Long => write!(f, "long"),
            BaseType::Short => write!(f, "short"),
            BaseType::Boolean => write!(f, "boolean"),
            BaseType::Void => write!(f, "void"),
        }
    }
}

pub type ObjectType = String;

#[derive(PartialEq, Debug, Clone)]
pub struct MethodType {
    pub parameter_types: Vec<ParameterType>,
    pub return_type: ReturnType,
}

impl std::fmt::Display for MethodType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut parameter_types = String::new();
        for parameter_type in &self.parameter_types {
            parameter_types.push_str(&format!("{}, ", parameter_type));
        }
        parameter_types.pop();
        parameter_types.pop();

        let return_type = match &self.return_type {
            Some(return_type) => format!("{}", return_type),
            None => String::from("Void"),
        };

        write!(f, "({}) -> {}", parameter_types, return_type)
    }
}

type ParameterType = FieldType;

type ReturnType = Option<FieldType>;

/// Parse a method descriptor into a MethodType.
pub fn parse_method_descriptor(descriptor: &String) -> MethodType {
    let mut parameter_types = Vec::new();

    let mut chars = descriptor.chars();
    let mut c = chars.next().unwrap();
    assert_eq!(c, '(');

    loop {
        c = chars.next().unwrap();
        if c == ')' {
            break;
        }

        let parameter_type = parse_field_type(&mut chars, c);
        parameter_types.push(parameter_type);
    }

    c = chars.next().unwrap();
    let return_type = parse_return_type(&mut chars, c);

    MethodType {
        parameter_types,
        return_type,
    }
}

/// Parse a field type descriptor into a FieldType.
pub fn parse_field_type_descriptor(descriptor: &String) -> FieldType {
    let mut chars = descriptor.chars();
    let c = chars.next().unwrap();
    parse_field_type(&mut chars, c)
}

fn parse_field_type(chars: &mut Chars, mut current: char) -> FieldType {
    match current {
        'B' => ParameterType::BaseType(BaseType::Byte),
        'C' => ParameterType::BaseType(BaseType::Char),
        'D' => ParameterType::BaseType(BaseType::Double),
        'F' => ParameterType::BaseType(BaseType::Float),
        'I' => ParameterType::BaseType(BaseType::Int),
        'J' => ParameterType::BaseType(BaseType::Long),
        'S' => ParameterType::BaseType(BaseType::Short),
        'Z' => ParameterType::BaseType(BaseType::Boolean),
        'V' => ParameterType::BaseType(BaseType::Void),
        'L' => {
            let mut object_type = String::new();
            loop {
                current = chars.next().unwrap();
                if current == ';' {
                    break;
                }
                object_type.push(current);
            }
            match &*object_type {
                "java/lang/Byte" => ParameterType::ObjectTypeJavaLangByte,
                "java/lang/Char" => ParameterType::ObjectTypeJavaLangChar,
                "java/lang/Double" => ParameterType::ObjectTypeJavaLangDouble,
                "java/lang/Float" => ParameterType::ObjectTypeJavaLangFloat,
                "java/lang/Integer" => ParameterType::ObjectTypeJavaLangInteger,
                "java/lang/Long" => ParameterType::ObjectTypeJavaLangLong,
                "java/lang/Short" => ParameterType::ObjectTypeJavaLangShort,
                "java/lang/Boolean" => ParameterType::ObjectTypeJavaLangBoolean,
                _ => ParameterType::ObjectType(object_type),
            }
        }
        '[' => {
            // Array
            current = chars.next().unwrap();
            let inner = parse_field_type(chars, current);
            ParameterType::ArrayType(Box::new(inner))
        }
        _ => unreachable!("{}", current),
    }
}

fn parse_return_type(chars: &mut Chars, current: char) -> ReturnType {
    match current {
        'V' => None,
        _ => {
            let field_type = parse_field_type(chars, current);
            Some(field_type)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_method_descriptor() {
        let descriptor = String::from("(Ljava/lang/String;I)V");
        let method_type = parse_method_descriptor(&descriptor);
        assert_eq!(method_type.parameter_types.len(), 2);
        assert_eq!(method_type.return_type, None);

        let descriptor = String::from("(Ljava/lang/String;I)I");
        let method_type = parse_method_descriptor(&descriptor);
        assert_eq!(method_type.parameter_types.len(), 2);
        assert_eq!(
            method_type.return_type,
            Some(FieldType::BaseType(BaseType::Int))
        );

        let descriptor = String::from("()V");
        let method_type = parse_method_descriptor(&descriptor);
        assert_eq!(method_type.parameter_types.len(), 0);
        assert_eq!(method_type.return_type, None);

        let descriptor = String::from("()I");
        let method_type = parse_method_descriptor(&descriptor);
        assert_eq!(method_type.parameter_types.len(), 0);
        assert_eq!(
            method_type.return_type,
            Some(FieldType::BaseType(BaseType::Int))
        );

        let descriptor = String::from("()Ljava/lang/String;");
        let method_type = parse_method_descriptor(&descriptor);
        assert_eq!(method_type.parameter_types.len(), 0);
        assert_eq!(
            method_type.return_type,
            Some(FieldType::ObjectType(String::from("java/lang/String")))
        );

        // Variable String array
        let descriptor = String::from("([Ljava/lang/String;)V");
        let method_type = parse_method_descriptor(&descriptor);
        assert_eq!(method_type.parameter_types.len(), 1);
        assert_eq!(
            method_type.parameter_types[0],
            ParameterType::ArrayType(Box::new(FieldType::ObjectType(String::from(
                "java/lang/String"
            )))),
        );
    }

    #[test]
    fn test_parse_field_type_descriptor() {
        let descriptor = String::from("Ljava/lang/String;");
        let field_type = parse_field_type_descriptor(&descriptor);
        assert_eq!(
            field_type,
            FieldType::ObjectType(String::from("java/lang/String"))
        );
    }

    #[test]
    fn test_java_lang_types_objects() {
        let descriptor = String::from("Ljava/lang/Byte;");
        let field_type = parse_field_type_descriptor(&descriptor);
        assert_eq!(field_type, FieldType::ObjectTypeJavaLangByte);

        let descriptor = String::from("Ljava/lang/Char;");
        let field_type = parse_field_type_descriptor(&descriptor);
        assert_eq!(field_type, FieldType::ObjectTypeJavaLangChar);

        let descriptor = String::from("Ljava/lang/Double;");
        let field_type = parse_field_type_descriptor(&descriptor);
        assert_eq!(field_type, FieldType::ObjectTypeJavaLangDouble);

        let descriptor = String::from("Ljava/lang/Float;");
        let field_type = parse_field_type_descriptor(&descriptor);
        assert_eq!(field_type, FieldType::ObjectTypeJavaLangFloat);

        let descriptor = String::from("Ljava/lang/Integer;");
        let field_type = parse_field_type_descriptor(&descriptor);
        assert_eq!(field_type, FieldType::ObjectTypeJavaLangInteger);

        let descriptor = String::from("Ljava/lang/Long;");
        let field_type = parse_field_type_descriptor(&descriptor);
        assert_eq!(field_type, FieldType::ObjectTypeJavaLangLong);

        let descriptor = String::from("Ljava/lang/Short;");
        let field_type = parse_field_type_descriptor(&descriptor);
        assert_eq!(field_type, FieldType::ObjectTypeJavaLangShort);

        let descriptor = String::from("Ljava/lang/Boolean;");
        let field_type = parse_field_type_descriptor(&descriptor);
        assert_eq!(field_type, FieldType::ObjectTypeJavaLangBoolean);
    }
}
