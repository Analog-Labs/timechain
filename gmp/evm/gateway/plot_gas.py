import csv
import matplotlib.pyplot as plt


class Gas:
    def __init__(self, row):
        self.num_msg = int(row[0])
        self.num_reg = int(row[1])
        self.num_unreg = int(row[2])
        self.msg_len = int(row[3])
        self.calldata_len = int(row[4])
        self.session_gas = int(row[5])
        self.execution_gas = int(row[6])

    def key(self):
        return (self.num_msg, self.num_reg, self.num_unreg, self.msg_len)

    def __getitem__(self, attr):
        return getattr(self, attr)


def read_csv(path):
    gas = {}
    with open(path, 'r') as f:
        r = csv.reader(f)
        r.__next__()
        for row in r:
            row = Gas(row)
            gas[row.key()] = row
    return gas


max_msg_size = 0x6000
gas = read_csv('gas.csv')
BASE = (0, 0, 0, 0)
REG = (0, 1, 0, 0)
UNREG = (0, 0, 1, 0)
MSG32 = (1, 0, 0, 32)
MSGMAX = (1, 0, 0, max_msg_size)


def slope(attr):
    return (gas[MSGMAX][attr] - gas[MSG32][attr]) / (gas[MSGMAX].msg_len - gas[MSG32].msg_len)

def offset(attr, slope):
    return gas[MSG32][attr] - slope * gas[MSG32].msg_len


print('calldata length')
print('===============')
calldata_len_base = gas[BASE].calldata_len
calldata_len_reg = gas[REG].calldata_len - calldata_len_base
calldata_len_unreg = gas[UNREG].calldata_len - calldata_len_base
calldata_len_msg = gas[MSG32].calldata_len - gas[MSG32].msg_len - calldata_len_base
calldata_len_msg_slope = 1
print('calldata_len_base', calldata_len_base)
print('calldata_len_reg', calldata_len_reg)
print('calldata_len_unreg', calldata_len_unreg)
print('calldata_len_msg', calldata_len_msg)
print('calldata_len_msg_slope', calldata_len_msg_slope)
print()

print('base gas')
print('========')
base_gas_base = calldata_len_base * 16 + 21000
base_gas_reg = calldata_len_reg * 16
base_gas_unreg = calldata_len_unreg * 16
base_gas_msg_slope = 16
base_gas_msg = calldata_len_msg * 16
print('base_gas_base', base_gas_base)
print('base_gas_reg', base_gas_reg)
print('base_gas_unreg', base_gas_unreg)
print('base_gas_msg', base_gas_msg)
print('base_gas_msg_slope', base_gas_msg_slope)
print()

print('session gas')
print('===========')
session_gas_base = gas[BASE].session_gas
session_gas_reg = gas[REG].session_gas - session_gas_base
session_gas_unreg = gas[UNREG].session_gas - session_gas_base
session_gas_msg_slope = slope('session_gas')
session_gas_msg = offset('session_gas', session_gas_msg_slope) - session_gas_base
print('session_gas_base', session_gas_base)
print('session_gas_reg', session_gas_reg)
print('session_gas_unreg', session_gas_unreg)
print('session_gas_msg', session_gas_msg)
print('session_gas_msg_slope', session_gas_msg_slope)
print()

print('execution gas')
print('=============')
execution_gas_base = gas[BASE].execution_gas
execution_gas_reg = gas[REG].execution_gas - execution_gas_base
execution_gas_unreg = gas[UNREG].execution_gas - execution_gas_base
execution_gas_msg_slope = slope('execution_gas')
execution_gas_msg = offset('execution_gas', execution_gas_msg_slope) - execution_gas_base
print('execution_gas_base', execution_gas_base)
print('execution_gas_reg', execution_gas_reg)
print('execution_gas_unreg', execution_gas_unreg)
print('execution_gas_msg', execution_gas_msg)
print('execution_gas_msg_slope', execution_gas_msg_slope)
print()

session_gas_base = session_gas_base + base_gas_base
session_gas_reg = session_gas_reg + base_gas_reg
session_gas_unreg = session_gas_unreg + base_gas_unreg
session_gas_msg = session_gas_msg + base_gas_msg
msg_slope = base_gas_msg_slope + session_gas_msg_slope + execution_gas_msg_slope

def session_gas(nr, nu, nm):
    return session_gas_base + session_gas_reg * nr + session_gas_unreg * nu + session_gas_msg * nm

def execution_gas(nr, nu, nm):
    return execution_gas_base + execution_gas_reg * nr + execution_gas_unreg * nu + execution_gas_msg * nm

def batch_gas(s, nr, nu, nm, msg_len, gas_limit):
    return (session_gas(nr, nu, nm) + msg_len * msg_slope) * s + execution_gas(nr, nu, nm) + gas_limit

def c0(s):
    return int(session_gas(0, 0, 1) * s + execution_gas(0, 0, 1))

def c1(s):
    return int(msg_slope * s)

print('send message constants')
print('======================')
print('s=%s c0=%s c1=%s' % (1, c0(1), c1(1)))
print('s=%s c0=%s c1=%s' % (2, c0(2), c1(2)))
print('s=%s c0=%s c1=%s' % (3, c0(3), c1(3)))
print()

print('gas')
print('===')
empty = int(batch_gas(1, 0, 0, 0, 0, 0))
reg = int(batch_gas(1, 1, 0, 0, 0, 0) - empty)
unreg = int(batch_gas(1, 0, 1, 0, 0, 0) - empty)
msg = int(batch_gas(1, 0, 0, 1, 0, 0) - empty)

print('empty', empty)
print('reg', reg)
print('unreg', unreg)
print('msg', msg)
