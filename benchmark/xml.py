import xml.etree.ElementTree as ET
from . import module_dir


def parse_countries():
    return ET.parse(module_dir / 'countries.xml')
