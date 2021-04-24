import shutil, sys, os

in_path = sys.argv[1]
if len(sys.argv) == 1:
    print("Specify file to package")
    exit()
elif len(sys.argv) == 2:
    out_path = in_path
else:
    out_path = sys.argv[2]
shutil.make_archive(out_path, "zip", in_path)
os.rename(out_path + ".zip", out_path + ".opk")