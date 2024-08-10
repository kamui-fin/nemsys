use anyhow::Result;
use serde::Deserialize;
use std::io::{BufRead, BufReader};
use std::iter::Skip;
use std::path::Path;
use std::vec::IntoIter;
use std::{
    fmt,
    fs::{self, File},
    path::PathBuf,
};

#[derive(Deserialize, Clone, fmt::Debug)]
pub struct MemTest(pub u16, pub u8); // address, data

#[derive(Deserialize, Clone, fmt::Debug, PartialEq)]
pub struct DatabusLog(pub u16, pub u8, pub String); // address, value, type

#[derive(Deserialize, Clone, fmt::Debug)]
pub struct CpuTestState {
    pub s: u8,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub p: u8,
    pub pc: u16,
    pub ram: Vec<MemTest>,
}

#[derive(Deserialize, Clone, fmt::Debug)]
pub struct InstructionTestCase {
    pub name: String,
    pub initial: CpuTestState,
    pub r#final: CpuTestState,
    pub cycles: Vec<DatabusLog>,
}

pub struct TestCaseIterator<I> {
    json_file_it: I,
}

pub struct TestCaseSet {
    pub opcode: u8,
    pub test_cases: Vec<InstructionTestCase>,
}

impl<I: Iterator<Item = PathBuf>> Iterator for TestCaseIterator<I> {
    type Item = TestCaseSet;

    fn next(&mut self) -> Option<Self::Item> {
        self.json_file_it.next().map(|path| {
            let unimplemented_opcodes: Vec<u8> = vec![
                0x4B, 0x0B, 0x2B, 0x8B, 0x6B, 0xBB, 0xAB, 0xCB, 0x9F, 0x93, 0x9E, 0x9C, 0x9B, 0x1A,
                0x3A, 0x5A, 0x7A, 0xDA, 0xFA, 0x80, 0x82, 0x89, 0xC2, 0xE2, 0x04, 0x44, 0x64, 0x14,
                0x34, 0x54, 0x74, 0xD4, 0xF4, 0x0C, 0x1C, 0x3C, 0x5C, 0x7C, 0xDC, 0xFC, 0x02, 0x12,
                0x22, 0x32, 0x42, 0x52, 0x62, 0x72, 0x92, 0xB2, 0xD2, 0xF2,
            ];
            let stem = path.file_stem().unwrap();
            let base_filename = stem.to_string_lossy();
            let opcode: u8 = u8::from_str_radix(&base_filename, 16).unwrap();
            if unimplemented_opcodes.contains(&opcode) {
                return TestCaseSet {
                    opcode,
                    test_cases: vec![],
                };
            }
            let json_text = fs::read_to_string(&path).unwrap();
            let test_cases: Vec<InstructionTestCase> = serde_json::from_str(&json_text).unwrap();
            TestCaseSet { opcode, test_cases }
        })
    }
}

pub fn load_json_tests(dir_path: &str) -> Result<TestCaseIterator<Skip<IntoIter<PathBuf>>>> {
    let entries = fs::read_dir(dir_path)?;
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .collect::<Vec<_>>();
    paths.sort();

    let target_index = if Path::new("/tmp/nemsys.ck").exists() {
        let file = File::open("/tmp/nemsys.ck")?;
        let mut reader = BufReader::new(file);
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let checkpoint_ins = line.trim();
        println!("Starting from {}", checkpoint_ins);

        paths
            .iter()
            .position(|p| {
                p.file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name == format!("{checkpoint_ins}.json"))
                    .unwrap_or(false)
            })
            .unwrap()
    } else {
        0
    };

    Ok(TestCaseIterator {
        json_file_it: paths.into_iter().skip(target_index),
    })
}
