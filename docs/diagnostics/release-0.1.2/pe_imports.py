"""Print the DLL names in a PE file's import table, like `objdump -p | grep "DLL Name"`.

Standard library only. Reads the import directory (data directory 1) and
maps RVAs to file offsets through the section table.
"""
import struct
import sys


def imports(path):
    data = open(path, "rb").read()
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    if data[pe:pe + 4] != b"PE\0\0":
        raise SystemExit(f"{path}: not a PE file")
    coff = pe + 4
    sections_count = struct.unpack_from("<H", data, coff + 2)[0]
    optional_size = struct.unpack_from("<H", data, coff + 16)[0]
    optional = coff + 20
    magic = struct.unpack_from("<H", data, optional)[0]
    directories = optional + (112 if magic == 0x20B else 96)
    import_rva = struct.unpack_from("<I", data, directories + 8)[0]
    sections = []
    table = optional + optional_size
    for index in range(sections_count):
        entry = table + index * 40
        virtual_size, virtual_address, raw_size, raw_pointer = struct.unpack_from("<IIII", data, entry + 8)
        sections.append((virtual_address, max(virtual_size, raw_size), raw_pointer))

    def offset(rva):
        for address, size, raw in sections:
            if address <= rva < address + size:
                return raw + rva - address
        raise ValueError(f"RVA {rva:#x} outside sections")

    names = []
    descriptor = offset(import_rva)
    while True:
        fields = struct.unpack_from("<IIIII", data, descriptor)
        if not any(fields):
            break
        start = offset(fields[3])
        names.append(data[start:data.index(b"\0", start)].decode("ascii"))
        descriptor += 20
    return names


for path in sys.argv[1:]:
    for name in sorted(imports(path)):
        print(f"DLL Name: {name}")
