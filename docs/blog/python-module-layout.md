```
❯ superflat main  ❯ python
Python 3.13.0 (main, Oct 16 2024, 03:23:02) [Clang 18.1.8 ] on linux
Type "help", "copyright", "credits" or "license" for more information.

>>> import superflat
>>> type(superflat)
>>> <class 'module'>
>>> from superflat import \_superflat
>>> type(\_superflat)
>>> <class 'module'>
>>> from \_superflat import pumpkin
>>> Traceback (most recent call last):
>>> File "<python-input-4>", line 1, in <module>

    from _superflat import pumpkin

ModuleNotFoundError: No module named '\_superflat'

>>> from superflat.\_superflat import pumpkin
>>> type(pumpkin)
>>> <class 'module'>
>>> from pumpkin import seed_from_level
>>> Traceback (most recent call last):
>>> File "<python-input-7>", line 1, in <module>

    from pumpkin import seed_from_level

ModuleNotFoundError: No module named 'pumpkin'

>>> from superflat.\_superflat.pumpkin import seed_from_level
>>> Traceback (most recent call last):
>>> File "<python-input-8>", line 1, in <module>

    from superflat._superflat.pumpkin import seed_from_level

ModuleNotFoundError: No module named 'superflat.\_superflat.pumpkin'; 'superflat.\_superflat' is not a package

>>> pumpkin.seed_from_level
>>> <built-in function seed_from_level>
>>>
```

怎么解释

---

● 这个行为是因为 \_superflat 是一个编译的 Rust 扩展模块，不是 Python 包。

让我解释：

1. from superflat import \_superflat - 成功，因为 Python 导入了 Rust 扩展
2. type(\_superflat) - 显示是 module，但这是一个原生扩展模块（用 Rust/C 编写）
3. 'superflat.\_superflat' is not a package - 关键错误：它不是包/目录，只是编译后的 .so 文件

Rust 扩展里面的东西（如 pumpkin）作为属性存在，而不是子模块。所以：

- pumpkin.seed_from_level 可以直接访问（Rust 扩展的导出函数）
- 但你不能用 from \_superflat import pumpkin 导入，因为 \_superflat 没有 Python 包结构

让我确认一下项目结构：

● Searched for 2 patterns, read 1 file (ctrl+o to expand)

● 现在清楚了。看 rust/src/lib.rs 第 14-21 行：

let pumpkin_pymodule = PyModule::new(py, "pumpkin")?;
pumpkin::init_submodule(&pumpkin_pymodule)?;
m.add_submodule(&pumpkin_pymodule)?;

let sys = py.import("sys")?;
sys.getattr("modules")?
.set_item("\_superflat.pumpkin", &pumpkin_pymodule)?;

代码尝试在运行时把 pumpkin 注册为子模块到 sys.modules，但这不够。

为什么不能 from superflat.\_superflat.pumpkin import ...：

1. \_superflat 是一个编译的 .so 扩展，不是 Python 包
2. Python 导入系统需要文件系统中的 **init**.py 或目录结构来识别包
3. 运行时手动添加到 sys.modules 只对直接属性访问有效（如 pumpkin.seed_from_level），但不支持完整路径的子模块导入

正确的导入方式：
from superflat.\_superflat import pumpkin
pumpkin.seed_from_level(...)

或者直接在 superflat 包层面导出。

✻ Cooked for 1m 26s
