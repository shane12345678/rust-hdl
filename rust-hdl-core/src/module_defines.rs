use crate::ast::{Verilog, VerilogLiteral};
use crate::atom::AtomKind::{StubInputSignal, StubOutputSignal};
use crate::atom::{Atom, AtomKind};
use crate::block::Block;
use crate::code_writer::CodeWriter;
use crate::named_path::NamedPath;
use crate::probe::Probe;
use crate::verilog_gen::verilog_combinatorial;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
struct SubModuleInvocation {
    kind: String,
    name: String,
}

#[derive(Clone, Debug, Default)]
struct ModuleDetails {
    atoms: Vec<AtomDetails>,
    sub_modules: Vec<SubModuleInvocation>,
    enums: Vec<EnumDefinition>,
    code: Verilog,
}

#[derive(Clone, Debug, PartialEq)]
struct EnumDefinition {
    pub type_name: String,
    pub discriminant: String,
    pub value: usize,
}

#[derive(Clone, Debug)]
struct AtomDetails {
    name: String,
    kind: AtomKind,
    width: usize,
    const_val: VerilogLiteral,
}

fn verilog_atom_name(x: &AtomKind) -> &str {
    match x {
        AtomKind::InputParameter => "input",
        AtomKind::OutputParameter => "output reg",
        AtomKind::StubInputSignal => "reg",
        AtomKind::StubOutputSignal => "wire",
        AtomKind::Constant => "localparam",
        AtomKind::LocalSignal => "wire",
    }
}

fn decl(x: &AtomDetails) -> String {
    if x.kind == AtomKind::Constant {
        format!(
            "{} {} = {};",
            verilog_atom_name(&x.kind),
            x.name,
            x.const_val
        )
    } else {
        if x.width == 1 {
            format!("{} {};", verilog_atom_name(&x.kind), x.name)
        } else {
            format!(
                "{} [{}:0] {};",
                verilog_atom_name(&x.kind),
                x.width - 1,
                x.name
            )
        }
    }
}

#[derive(Default)]
pub struct ModuleDefines {
    path: NamedPath,
    namespace: NamedPath,
    details: BTreeMap<String, ModuleDetails>,
}

impl ModuleDefines {
    fn add_atom(&mut self, module: &str, atom: AtomDetails) {
        let entry = self.details.entry(module.into()).or_default();
        entry.atoms.push(atom)
    }
    fn add_submodule(&mut self, module: &str, name: &str, kind: &str) {
        let entry = self.details.entry(module.into()).or_default();
        entry.sub_modules.push(SubModuleInvocation {
            kind: kind.to_owned(),
            name: name.to_owned(),
        });
    }
    fn add_enum(&mut self, module: &str, signal: &dyn Atom) {
        let entry = self.details.entry(module.into()).or_default();
        let enum_name = signal.type_name();
        let enum_values = (0..(1 << signal.bits()))
            .map(|x| EnumDefinition {
                type_name: enum_name.into(),
                discriminant: signal.name(x).into(),
                value: x,
            })
            .filter(|x| x.discriminant.len() != 0)
            .collect::<Vec<_>>();
        entry.enums.extend(enum_values.into_iter())
    }
    fn add_code(&mut self, module: &str, code: Verilog) {
        let entry = self.details.entry(module.into()).or_default();
        entry.code = code;
    }
}

impl Probe for ModuleDefines {
    fn visit_start_scope(&mut self, name: &str, node: &dyn Block) {
        let top_level = self.path.to_string();
        self.path.push(name);
        self.namespace.reset();
        self.add_submodule(&top_level, name, &self.path.to_string());
        self.add_code(&self.path.to_string(), node.hdl());
    }

    fn visit_start_namespace(&mut self, name: &str, _node: &dyn Block) {
        self.namespace.push(name);
    }

    fn visit_atom(&mut self, name: &str, signal: &dyn Atom) {
        println!(
            "Atom: name {} path {} namespace {} enum {} type {}",
            name,
            self.path.to_string(),
            self.namespace.flat("_"),
            signal.is_enum(),
            signal.type_name()
        );
        let module_path = self.path.to_string();
        let module_name = self.path.last();
        let namespace = self.namespace.flat("_");
        let name = if namespace.is_empty() {
            name.to_owned()
        } else {
            format!("{}_{}", namespace, name)
        };
        let param = AtomDetails {
            name: name.clone(),
            kind: signal.kind(),
            width: signal.bits(),
            const_val: signal.verilog(),
        };
        if param.kind.is_parameter() {
            let kind = if param.kind == AtomKind::InputParameter {
                StubInputSignal
            } else {
                StubOutputSignal
            };
            let parent_param = AtomDetails {
                name: format!("{}_{}", module_name, name.to_owned()),
                kind,
                width: signal.bits(),
                const_val: signal.verilog(),
            };
            let parent_name = self.path.parent();
            self.add_atom(&parent_name, parent_param);
        }
        if signal.is_enum() {
            self.add_enum(&module_path, signal);
        }
        self.add_atom(&module_path, param);
    }

    fn visit_end_namespace(&mut self, _name: &str, _node: &dyn Block) {
        self.namespace.pop();
    }

    fn visit_end_scope(&mut self, _name: &str, _node: &dyn Block) {
        self.path.pop();
    }
}

impl ModuleDefines {
    pub fn defines(&self) -> String {
        let mut io = CodeWriter::new();
        self.details
            .iter()
            .filter(|x| x.0.len() != 0)
            .filter(|x| !matches!(x.1.code, Verilog::Blackbox(_)))
            .for_each(|k| {
                let module_name = k.0;
                let module_details = k.1;
                let atoms = &module_details.atoms;
                let args = atoms
                    .iter()
                    .filter(|x| x.kind.is_parameter())
                    .collect::<Vec<_>>();
                let stubs = atoms
                    .iter()
                    .filter(|x| x.kind.is_stub())
                    .collect::<Vec<_>>();
                let consts = atoms
                    .iter()
                    .filter(|x| x.kind == AtomKind::Constant)
                    .collect::<Vec<_>>();
                let locals = atoms
                    .iter()
                    .filter(|x| x.kind == AtomKind::LocalSignal)
                    .collect::<Vec<_>>();
                let module_args = args
                    .iter()
                    .map(|x| x.name.to_owned())
                    .collect::<Vec<_>>()
                    .join(",");
                io.add(format!("\n\nmodule {}({});", module_name, module_args));
                io.push();
                if !args.is_empty() {
                    io.add("\n// Module arguments");
                    args.iter().for_each(|x| io.add(decl(x)));
                }
                let submodules = &module_details.sub_modules;
                if !consts.is_empty() {
                    io.add("\n// Constant declarations");
                    consts.iter().for_each(|x| io.add(decl(x)));
                }
                if !module_details.enums.is_empty() {
                    io.add("\n// Enums");
                    module_details.enums.iter().for_each(|x| {
                        io.add(format!(
                            "localparam {}_{} = {}",
                            x.type_name, x.discriminant, x.value
                        ))
                    });
                }
                if !stubs.is_empty() {
                    io.add("\n// Stub signals");
                    stubs.iter().for_each(|x| io.add(decl(x)));
                }
                if !locals.is_empty() {
                    io.add("\n// Local signals");
                    locals.iter().for_each(|x| io.add(decl(x)));
                }
                if !submodules.is_empty() {
                    io.add("\n// Sub module instances");
                    for child in submodules {
                        let entry = self.details.get(&child.kind).unwrap();
                        if !matches!(entry.code, Verilog::Blackbox(_)) {
                            let child_args = entry
                                .atoms
                                .iter()
                                .filter(|x| {
                                    x.kind == AtomKind::InputParameter
                                        || x.kind == AtomKind::OutputParameter
                                })
                                .map(|x| format!(".{}({}_{})", x.name, child.name, x.name))
                                .collect::<Vec<_>>()
                                .join(",");
                            io.add(format!("{} {}({});", child.kind, child.name, child_args))
                        }
                    }
                }
                match &module_details.code {
                    Verilog::Combinatorial(code) => {
                        io.add("\n// Update code");
                        io.add(verilog_combinatorial(code));
                    }
                    Verilog::Custom(code) => {
                        io.add("\n// Update code (custom)");
                        io.add(code);
                    }
                    Verilog::Blackbox(_) => {}
                    Verilog::Empty => {}
                }
                io.pop();
                io.add(format!("endmodule // {}", module_name));
            });
        self.details
            .iter()
            .filter(|x| matches!(x.1.code, Verilog::Blackbox(_)))
            .for_each(|k| {
                if let Verilog::Blackbox(b) = &k.1.code {
                    io.add(b)
                }
            });
        io.to_string()
    }
}

pub fn generate_verilog<U: Block>(uut: &U) -> String {
    let mut defines = ModuleDefines::default();
    uut.accept("top", &mut defines);
    defines.defines()
}
