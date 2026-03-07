from collections import Counter
from hashlib import sha256
from pathlib import Path
from random import randint, seed

import structlog
from pumpkin_py import generate_chunk_nbt, normalize_nbt

SECTOR_SIZE = 4096

log = structlog.get_logger()


def random_test():
    log.info("running random test")
    seed(42)
    Path("temp").mkdir(exist_ok=True)
    for i in range(1000):
        world_seed = randint(0, 1_000_000_000)
        chunk_x = randint(-1000, 1000)
        chunk_z = randint(-1000, 1000)
        log.info(
            "test case running",
            world_seed=world_seed,
            chunk_x=chunk_x,
            chunk_z=chunk_z,
            iter=i,
        )
        result1 = normalize_nbt(generate_chunk_nbt(world_seed, chunk_x, chunk_z))
        result2 = normalize_nbt(generate_chunk_nbt(world_seed, chunk_x, chunk_z))
        if not result1 == result2:
            with open(f"temp/{i}-a.nbt", "wb") as f:
                f.write(result1)
            with open(f"temp/{i}-b.nbt", "wb") as f:
                f.write(result2)
            log.error(
                "assert eq failed",
                world_seed=world_seed,
                chunk_x=chunk_x,
                chunk_z=chunk_z,
            )


def brute_force_test():
    log.info("running brute force test")
    hash_set: set[bytes] = set()
    hash_list = []
    i = 0
    for i in range(3000):
        chunk_x = 669
        chunk_z = 473
        world_seed = 657830420
        result = generate_chunk_nbt(world_seed, chunk_x, chunk_z)
        assert isinstance(result, bytes)
        hash = sha256(result).digest()
        hash_list.append(hash)

        if hash in hash_set:
            pass
        else:
            log.info(
                "found",
                world_seed=world_seed,
                chunk_x=chunk_x,
                chunk_z=chunk_z,
                iteration=i,
                short_hash=hash[:4].hex(),
            )
            hash_set.add(hash)
            with open(f"iter-{i}.nbt", "wb") as f:
                f.write(result)
        i += 1
    log.info(f"hash counter: {Counter(hash_list)}")


def single_case_test():
    log.info("running single case test")
    chunk_x = 669
    chunk_z = 473
    world_seed = 657830420
    result1 = generate_chunk_nbt(world_seed, chunk_x, chunk_z)
    assert isinstance(result1, bytes)
    # with open("out1.nbt", "wb") as f:
    #     f.write(result1)
    result2 = generate_chunk_nbt(world_seed, chunk_x, chunk_z)
    assert isinstance(result2, bytes)
    # with open("out2.nbt", "wb") as f:
    #     f.write(result2)
    assert result1 == result2


def main(log_level: str = "info"):
    structlog.configure(
        wrapper_class=structlog.make_filtering_bound_logger(log_level),
    )
    random_test()


if __name__ == "__main__":
    main()
