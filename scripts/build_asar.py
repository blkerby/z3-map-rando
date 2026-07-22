# Adapted from PJBoy's script: https://github.com/blkerby/MapRandomizer/blob/main/python/scripts/build_asar.py
import argparse, os, pathlib, subprocess

os.chdir(pathlib.Path(__file__).parent.parent)

argparser = argparse.ArgumentParser(description = 'Build asar')
argparser.add_argument('-j', action = 'store_true', help = 'Enable build parallelisation')
argparser.add_argument('--clean', action = 'store_true', help = 'Do a clean build')
argparser.add_argument('--reset', action = 'store_true', help = 'Hard reset asar submodule first')
args = argparser.parse_args()

if args.reset:
    subprocess.run(['git', '-C', 'asar', 'reset', '--hard'], check = True)

if args.clean:
    subprocess.run(['git', '-C', 'asar', 'clean', '-dfx'], check = True)

# Specify release here for single-config generators (ignored by multi-config generators)
subprocess.run(['cmake', 'asar/src', '-B', 'asar/build', '-DCMAKE_BUILD_TYPE=Release'], check = True)

# Specify release here for multi-config generators (ignored by single-config generators)
build_cmd = ['cmake', '--build', 'asar/build', '--config', 'Release']
if args.j:
    build_cmd += ['-j']

subprocess.run(build_cmd, check = True)

# For multi-config generators, move the output binary to the location single-config generators use
dir_path = pathlib.Path('asar/build/asar/bin/Release/')
if dir_path.exists():
    asar_path = dir_path / 'asar'
    if not asar_path.exists():
        asar_path = dir_path / 'asar.exe'
        if not asar_path.exists():
            raise RuntimeError('Could not resolve path to asar binary')
    
    asar_path.move_into('asar/build/asar/bin/')
