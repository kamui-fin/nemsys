import subprocess
from pprint import pprint
import re


def run_cargo_command():
    result = subprocess.run(
        ["cargo", "run", "--bin", "test_cpu"], capture_output=True, text=True
    )
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
    emulator_logs = read_logs("nemsys.log")
    ground_truth_logs = read_logs("romtest/nestest.log")
    error = compare_logs(emulator_logs, ground_truth_logs)
    if not error:
        print("No errors found!")
        return
    a, b = error
    i = 0
    print("Context:")
    print(format_rust_log(context[i][0]), end="")
    print(context[i][1])

    print("Error at:")
    print(format_rust_log(a), end="")
    print(b)
    print("-----")


main()
