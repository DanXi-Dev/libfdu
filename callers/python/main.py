import ctypes
import unittest

import utils

# Please install psutil to get the memory usage of the process.

DLL = ctypes.cdll.LoadLibrary("../../target/debug/libfdu.dll")

DLL.hello_world.restype = ctypes.c_void_p
DLL.free_string.argtypes = (ctypes.c_void_p,)


class TestMain(unittest.TestCase):

    def test_add(self):
        res = DLL.add(1, 2)
        self.assertEqual(res, 3)

    def test_hello_world(self):
        ptr = DLL.hello_world()
        try:
            res = ctypes.cast(ptr, ctypes.c_char_p).value.decode()
        finally:
            DLL.free_string(ptr)
            pass
        self.assertEqual(res, 'hello world')

    def test_memory_leak(self):
        start = utils.get_memory_usage()
        for i in range(10 ** 4):
            self.test_hello_world()
        end = utils.get_memory_usage()
        usage = abs(end - start)
        print(f"Memory usage: {usage} bytes")
        self.assertLess(usage, 10 ** 4)


if __name__ == '__main__':
    unittest.main()
