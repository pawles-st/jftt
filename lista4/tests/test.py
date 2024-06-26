import re
from os import chdir
from subprocess import run, check_output, Popen, PIPE, CalledProcessError

class CompilerException(Exception):
    pass

class ExecutionException(Exception):
    pass

ansi_escape_8bit = re.compile(
    br'(?:\x1B[@-Z\\-_]|[\x80-\x9A\x9C-\x9F]|(?:\x1B\[|\x9B)[0-?]*[ -/]*[@-~])'
)

#curr_dir = popen("pwd")
#print(curr_dir.read())

examples1 = ["example" + str(i) + ".imp" for i in range(1, 10)]
examples1_data = ["37, 15", "0, 1", "1", "20, 9", "1234567890, 1234567890987654321, 987654321", "20", "0, 0, 0", "", "20, 9"]
examples1_expected = ["13, 32, 1", "46368, 28657", "121393", "167960", "674106858", "2432902008176640000, 6765", "31000, 40900, 2222010", "5, 2, 10, 4, 20, 8, 17, 16, 11, 9, 22, 18, 21, 13, 19, 3, 15, 6, 7, 12, 14, 1, 0, 1234567890, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22", "167960"]

examples2 = ["program" + str(i) + ".imp" for i in range(0, 4)]
examples2_data = ["11", "4, 8, 32, 1070", "", "1234567890"]
examples2_expected = ["1, 1, 0, 1", "2", "2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97", "2, 1, 3, 2, 5, 1, 3607, 1, 3803, 1"]

examples3 = ["test" + str(i) + ".imp" for i in range(1, 5)]
examples3_data = ["", "", "", ""]
examples3_expected = ["1, 1, 1", "1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1", "14, 8, 33, 3, 2, 0, 0, 0, 0", "1, 10, 10"]

examples4 = ["try" + str(i) + ".imp" for i in range(1, 3)]
examples4_data = ["", ""]
examples4_expected = ["1, 2, 3", "92233720368547758075, 166020696663385964535"]

#examples5 = ["test0.imp", "test1.imp", "test2a.imp", "test2b.imp", "test2c.imp", "test2d.imp"]
#examples5_data = ["340282367713220089251654026161790386200, 340282367713220089251654026161790386200", "", "", "1234567890"]
#examples5_expected = ["1, 1, 0, 1", "2", "2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97", "2, 1, 3, 2, 5, 1, 3607, 1, 3803, 1"]

# correct tests

programs = examples1 + examples2 + examples3 + examples4
programs_data = examples1_data + examples2_data + examples3_data + examples4_data
programs_expected = examples1_expected + examples2_expected + examples3_expected + examples4_expected

for i in range(0, len(programs)):
    chdir("../compiler")
    compiler_result = run("cargo run -q ../" + programs[i] + " ../code.mr 2>/dev/null", shell=True)
    if compiler_result.returncode != 0:
        raise CompilerException(f"invalid compilation for {programs[i]}")

    chdir("../maszyna_wirtualna")
    vm_result = run("./maszyna-wirtualna-cln ../code.mr", check=True, stdout=PIPE, shell=True, input=programs_data[i].replace(", ", "\n"), encoding="utf-8")
    print(programs[i])
    #vm_result.stdout = ansi_escape_8bit.sub(b'', bytes(vm_result.stdout, encoding="utf-8"))
    commands_line = vm_result.stdout.split("\n")[1]
    print(commands_line)
    #commands = int(commands_line.split(" ")[-1][:-2])
    cost_line = vm_result.stdout.split("\n")[-2]
    print(cost_line)
    #cost = int(cost_line[(cost_line.find("koszt:")+7):cost_line.find(";")])
    #print(commands, cost)
    output = vm_result.stdout.split("> ")[1:];
    results = ", ".join([out.split("\n")[0] for out in output])
    if results != programs_expected[i]:
        raise ExecutionException(f"invalid result for {programs[i]}: got {results}, expected {programs_expected[i]}")

# error tests

errors = ["error" + str(i) + ".imp" for i in range(1, 9)]
#errors_expected = [""" Error: No such variable: "e" """, """  """]

programs = errors
#programs_expected = errors_expected

for i in range(0, len(programs)):
    chdir("../compiler")
    try:
        compiler_result = run("cargo run -q ../" + programs[i] + " ../code.mr", check=True, stderr=PIPE, shell=True, encoding="utf-8")
    except CalledProcessError:
        pass
    else:
        raise CompilerException(f"invalid correct compilation for {programs[i]}")

print("TESTS SUCCEEDED")

