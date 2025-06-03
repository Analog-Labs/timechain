import csv
import matplotlib.pyplot as plt


def read_csv(path):
    data = {}
    with open(path, 'r') as f:
        r = csv.reader(f)
        r.__next__()
        for row in r:
            data[int(row[0])] = (int(row[1]), int(row[2]), int(row[3]))
        data = dict(sorted(data.items()))
    x, y0, y1, y2 = [], [], [], []
    for (k, (v0, v1, v2)) in data.items():
        x.append(k)
        y0.append(v0)
        y1.append(v1)
        y2.append(v2)
    return (x, y0, y1, y2)


def lin(x, y):
    slope = (y[-1] - y[0]) / (x[-1] - x[0])
    offset = y[0] - slope * x[0]
    return (offset, slope)


def plot(c0, c1, max_x):
    xp = [x in range(0, max_x, 100)]
    yp = [c1 * x + c0 for x in xp]
    plt.plot(xp, yp)


msg_size, exec, reimb, base = read_csv('gas.csv')
#print(msg_size, exec, reimb, base)
c_exec = lin(msg_size, exec)
c_reimb = lin(msg_size, reimb)
c_base = lin(msg_size, base)
print('measured functions')
print('==================')
print('exec(msg_size, gas_limit):', c_exec[0], '+', c_exec[1], '*', 'msg_size', '+', 'gas_limit')
print('reimb(msg_size):', c_reimb[0], '+', c_reimb[1], '*', 'msg_size')
print('base(msg_size):', c_base[0], '+', c_base[1], '*', 'msg_size')
print()

# cost per session
cb = c_base[0] + c_reimb[0]
# cost per session and message byte
cm = c_base[1] + c_exec[1]
# cost of execution
cd = c_exec[0] - c_reimb[0]

print('sessions size independent constants')
print('===================================')
print('session cost (cb)', cb)
print('message byte cost (cm)', cm)
print('execution cost (cd)', cd)
print()

def coefs(s):
    c0 = int(s * cb + cd)
    c1 = int(s * cm)
    print('session_size', s, 'c0', c0, 'c1', c1)
    return (s, c0, c1)

# number of sessions for shard
def sessions(n, t):
    return n - t + 1

print('session size dependent constants')
print('================================')
s1 = coefs(sessions(1, 1))
s2 = coefs(sessions(3, 2))
s3 = coefs(sessions(6, 4))
print()

def msg_gas(s, msg_size):
    gas = s[2] * msg_size + s[1]
    print('gas(s=%s, x=%s) = %s' % (s[0], msg_size, gas))

print('gas for message size')
print('====================')
max_msg_size = 0x6000
msg_gas(s1, 0)
msg_gas(s1, max_msg_size)
msg_gas(s2, 0)
msg_gas(s2, max_msg_size)
msg_gas(s3, 0)
msg_gas(s3, max_msg_size)
