import subprocess
from pprint import pprint
import re


def run_cargo_command():
    result = subprocess.run(["cargo", "run"], capture_output=True, text=True)
    if result.returncode != 0:
        print(f"Error running 'cargo run': {result.stderr}")
        return None
    return result.stdout


def read_logs(file_path):
    with open(file_path, "r") as file:
        return file.readlines()


def parse_emulator_log(line):
    match = re.match(
        r".*\[INFO\] (\w+)\s+(\w+)\s+A:(\w+) X:(\w+) Y:(\w+) P:(\w+) SP:(\w+)", line
    )
    if match:
        return [int(i, 16) for i in match.groups()]
    return None


def parse_ground_truth_log(line):
    match = re.match(
        r"(\w+)\s+(\w+)\s+.*\s+A:(\w+) X:(\w+) Y:(\w+) P:(\w+) SP:(\w+).*", line
    )
    if match:
        return [int(i, 16) for i in match.groups()]
    return None


context = {}
unofficial_opcodes = set(
    [
        0x4B,
        0x0B,
        0x2B,
        0x8B,
        0x6B,
        0xC7,
        0xD7,
        0xCF,
        0xDF,
        0xDB,
        0xC3,
        0xD3,
        0xE7,
        0xF77,
        0xEF,
        0xFF,
        0xFB,
        0xE3,
        0xF3,
        0xBB,
        0xA7,
        0xB7,
        0xB7,
        0xAF,
        0xBF,
        0xA3,
        0xB3,
        0xAB,
        0x27,
        0x37,
        0x2F,
        0x3F,
        0x3B,
        0x23,
        0x33,
        0x67,
        0x77,
        0x6F,
        0x7F,
        0x7B,
        0x63,
        0x73,
        0x87,
        0x97,
        0x8F,
        0x83,
        0xCB,
        0x9F,
        0x93,
        0x9E,
        0x9C,
        0x07,
        0x17,
        0x0F,
        0x1F,
        0x1B,
        0x03,
        0x13,
        0x47,
        0x57,
        0x4F,
        0x5F,
        0x5B,
        0x43,
        0x53,
        0x9B,
        0xEB,
        0x1A,
        0x3A,
        0x5A,
        0x7A,
        0xDA,
        0xFA,
        0x80,
        0x82,
        0x89,
        0xC2,
        0xE2,
        0x04,
        0x44,
        0x64,
        0x14,
        0x34,
        0x54,
        0x74,
        0xD4,
        0xF4,
        0x0C,
        0x1C,
        0x3C,
        0x5C,
        0x7C,
        0xDC,
        0xFC,
        0x02,
        0x12,
        0x22,
        0x32,
        0x42,
        0x52,
        0x62,
        0x72,
        0x92,
        0xB2,
        0xD2,
        0xF2,
    ]
)


# Function to compare logs
def compare_logs(emulator_logs, ground_truth_logs):
    discrepancies = []
    i, j = 0, 0
    while i < len(emulator_logs) and j < len(ground_truth_logs):
        emu_log, gt_log = emulator_logs[i], ground_truth_logs[j]

        emu_parsed = parse_emulator_log(emu_log)
        gt_parsed = parse_ground_truth_log(gt_log)

        if not emu_parsed:
            i += 1
            continue

        if gt_parsed[1] in unofficial_opcodes:
            i += 1
            j += 1
            continue

        if emu_parsed[-2] | 0x10 == gt_parsed[-2] | 0x10:
            emu_parsed[-2] = gt_parsed[-2]

        if emu_parsed != gt_parsed:
            context[len(discrepancies)] = (
                emulator_logs[i - 1],
                ground_truth_logs[j - 1],
            )
            return (emu_log, gt_log)

        i += 1
        j += 1


def format_rust_log(s):
    return s


def main():
    run_cargo_command()
    emulator_logs = read_logs("xines.log")
    ground_truth_logs = read_logs("romtest/nestest.log")
    for i, (a, b) in enumerate([compare_logs(emulator_logs, ground_truth_logs)]):
        print("Context:")
        print(format_rust_log(context[i][0]), end="")
        print(context[i][1])

        print("Error at:")
        print(format_rust_log(a), end="")
        print(b)
        print("-----")
        break


main()
