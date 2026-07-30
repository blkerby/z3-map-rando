import glob, os, pathlib, subprocess

os.chdir(pathlib.Path(__file__).parent.parent)

for asm_path in glob.iglob('patches/src/*.asm'):
    filename = pathlib.Path(asm_path).parts[-1]
    ips_path = pathlib.Path('patches/ips').joinpath(filename).with_suffix('.ips')
    print("Building", asm_path)
    subprocess.run([
        'asar/build/asar/bin/asar', 
        '--fix-checksum=off', 
        '--no-title-check', 
        '--disable-read', 
        '--ips', ips_path,
        asm_path, 
        '/tmp/dummy.smc'
    ], check=True)
