import re
import subprocess
import time
from pathlib import Path
from tempfile import TemporaryDirectory
from typing import Iterator

import structlog
import typer
from platformdirs import user_cache_path
from podman import PodmanClient
from podman.domain.containers import Container
from podman.domain.images import Image
from podman.errors import ImageNotFound

from superflat.flatten import flatten_mca

SECTOR_SIZE = 4096

log = structlog.get_logger()


def main():
    log.info("Hello from superflat!")
    prototype("1.21.4", "3055925211550632933", (-2, -2), (1, 1))


def wait_logs_until(logs: Iterator[bytes], stop_pattern: str):
    pattern = re.compile(stop_pattern)
    for line in logs:
        line = line.decode()
        log.debug("waiting stop pattern", stop_pattern=stop_pattern, log_line=line)
        if pattern.search(line):
            break


def get_container_name(version: str, seed: str) -> str:
    return f"superflat-save-generator-{version}-{seed}"


def generate_save(
    version: str,
    seed: str,
    corner1: tuple[int, int],
    corner2: tuple[int, int],
    dst: Path,
):
    if dst.exists():
        if not dst.is_dir():
            raise ValueError("dst must be a dir path")
    else:
        dst.mkdir(parents=True)
    x1 = min(corner1[0], corner2[0]) * 16
    z1 = min(corner1[1], corner2[1]) * 16
    x2 = (max(corner1[0], corner2[0]) + 1) * 16 - 1
    z2 = (max(corner1[1], corner2[1]) + 1) * 16 - 1
    log.info(
        "starting save generation",
        version=version,
        seed=seed,
        x1=x1,
        z1=z1,
        x2=x2,
        z2=z2,
        dst=dst,
    )
    PODMAN_API_URL = "tcp://localhost:8888"
    REPOSITORY = "docker.io/itzg/minecraft-server"
    CONTAINER_NAME = get_container_name(version, seed)

    podman_service = subprocess.Popen(
        ["podman", "system", "service", "--time=0", PODMAN_API_URL],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    time.sleep(3)
    log.debug("podman service started", pid=podman_service.pid)

    container: Container | None = None
    try:
        with PodmanClient(base_url=PODMAN_API_URL) as client:
            log.info("podman client ping")
            if client.ping():
                log.info("podman client pong")
            else:
                raise RuntimeError("Ping Podman client failed")

            if not client.images.exists(REPOSITORY):
                log.info("pulling image", repository=REPOSITORY)
                client.images.pull(REPOSITORY)
            image = client.images.get(REPOSITORY)

            if not client.containers.exists(CONTAINER_NAME):
                log.info("creating container")
                client.containers.create(
                    image=image,
                    name=CONTAINER_NAME,
                    ports={"25565/tcp": 25565},
                    environment={
                        "EULA": "TRUE",
                        "TYPE": "FABRIC",
                        "VERSION": version,
                        "MEMORY": "4G",
                        "MODRINTH_PROJECTS": "fabric-api\nchunky",
                        "ONLINE_MODE": "FALSE",
                        "GENERATE_STRUCTURES": "TRUE",
                        "SEED": seed,
                    },
                    restart_policy={"Name": "unless-stopped"},
                )
            container = client.containers.get(CONTAINER_NAME)

            log.info("starting container", status=container.status)
            container.start()
            start_time = int(time.time())
            log.info(
                "container started",
                container_id=container.short_id,
                start_time=start_time,
            )

            try:
                logs: Iterator[bytes] = container.logs(
                    timestamps=True, stream=True, follow=True, since=start_time
                )  # pyright: ignore[reportAssignmentType]

                log.info("waiting until server started")
                wait_logs_until(logs, r'Done \(\d+\.?\d+s\)! For help, type "help"')
                log.info("waiting until rcon listener started")
                wait_logs_until(logs, r"Thread RCON Listener started")

                rcon_commands = [
                    (
                        ["rcon-cli", f"chunky corners {x1} {z1} {x2} {z2}"],
                        None,
                    ),
                    (["rcon-cli", "chunky start"], r"\[Chunky\] Task finished"),
                    (
                        ["rcon-cli", "save-all flush"],
                        r"ThreadedAnvilChunkStorage: All dimensions are saved",
                    ),
                ]
                for cmd, stop_pattern in rcon_commands:
                    log.info(
                        "running rcon command", command=cmd, stop_pattern=stop_pattern
                    )
                    exec_run_result: tuple[int, tuple[bytes | None, bytes | None]] = (  # pyright: ignore[reportAssignmentType]
                        container.exec_run(cmd, stdout=True, stderr=True, demux=True)
                    )
                    log.debug("exec_run result", exec_run_result=exec_run_result)
                    (exit_code, (stdout, stderr)) = exec_run_result

                    log.debug(
                        "RCON command exited",
                        command=cmd,
                        exit_code=exit_code,
                        stdout=stdout,
                        stderr=stderr,
                    )
                    if exit_code != 0:
                        raise RuntimeError("RCON command failed")
                    if stop_pattern:
                        wait_logs_until(logs, stop_pattern)

                log.info("copying region file")
                copy_result = subprocess.run(
                    [
                        "podman",
                        "cp",
                        f"{container.id}:/data/world/region/",
                        str(dst),
                    ],
                    capture_output=True,
                    text=True,
                )

                if copy_result.returncode != 0:
                    log.debug(
                        "failed to copy region file",
                        container_id=container.short_id,
                        stdout=copy_result.stdout,
                        stderr=copy_result.stderr,
                    )
                    raise RuntimeError("Failed to copy region file")

            except Exception as e:
                log.exception("error in save generation", err=e)
                raise
            finally:
                log.info("stopping container", container_id=container.short_id)
                container.stop()
                container = None

    except Exception as e:
        log.exception("error in save generation", err=e)
        raise
    finally:
        log.debug("terminating podman service")
        podman_service.terminate()
        podman_service.wait(timeout=30)
        log.info("save generation complete")


def get_prototype_path(version: str, seed: str):
    return (
        user_cache_path("superflat", "HairlessVillager") / "prototype" / version / seed
    )


def normalize_save(src: Path, dst: Path):
    for src_mca_filepath in src.glob("**/*.mca"):
        relative_mca_filepath = src_mca_filepath.relative_to(src)
        dst_mca_mcc_filepath = (dst / relative_mca_filepath).parent
        flatten_mca(src_mca_filepath, dst_mca_mcc_filepath)


def prototype(
    version: str, seed: str, corner1: tuple[int, int], corner2: tuple[int, int]
):
    with TemporaryDirectory("-superflat", delete=False) as td:
        temp_dir = Path(td)
        cache_dir = get_prototype_path(version, seed)
        log.info("working temp_dir", temp_dir=temp_dir)
        generate_save(version, seed, corner1, corner2, dst=temp_dir)
        # normalize_save(temp_dir, cache_dir)


def cli():
    typer.run(main)


if __name__ == "__main__":
    main()
