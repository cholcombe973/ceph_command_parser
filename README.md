# ceph_command_parser
https://github.com/ceph/ceph/blob/master/src/mon/MonCommands.h contains the list of all possible Ceph commands that can be sent
to the cluster.  This utility will parse that file and build Python code to talk to Ceph with the API commands.  

Running the code generally goes like this:
  - install python yapf
  - `cargo build`
  - `cat /tmp/MonCommands.h | ./target/debug/command_parser | yapf > ceph_command.py`
  - git commit the ceph_command.py and push it
