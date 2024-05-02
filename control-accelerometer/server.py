import socket
import struct

MCAST_GRP = '224.0.0.123'
MCAST_PORT = 7645
IS_ALL_GROUPS = '0.0.0.0'

sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_UDP)
sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
sock.bind((IS_ALL_GROUPS, MCAST_PORT))
mreq = struct.pack("4sl", socket.inet_aton(MCAST_GRP), socket.INADDR_ANY)
sock.setsockopt(socket.IPPROTO_IP, socket.IP_ADD_MEMBERSHIP, mreq)

# Keep it running to receive messages
while True:
  print(sock.recv(10240))
