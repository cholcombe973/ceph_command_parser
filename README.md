# ceph_command_parser
https://github.com/ceph/ceph/blob/master/src/mon/MonCommands.h contains the list of all possible Ceph commands that can be sent 
to the cluster.  This utility binary will parse that file and build Python code to talk to Ceph with the API commands.
