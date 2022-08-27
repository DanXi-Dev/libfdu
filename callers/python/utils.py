import os

import psutil


def get_memory_usage():
    return psutil.Process(os.getpid()).memory_info().rss
