import re
import subprocess
import time
from pathlib import Path
from typing import Iterator

import structlog
import typer
from podman import PodmanClient
from podman.domain.containers import Container

SECTOR_SIZE = 4096

log = structlog.get_logger()


def main():
    log.info("Hello from superflat!")
    prototype("1.21.4", "3055925211550632933", (-2, -2), (1, 1))


def wait_logs_until(logs: Iterator[bytes], stop_pattern: str):
    pattern = re.compile(stop_pattern)
    for line in logs:
        line = line.decode()
        log.debug("container log", line=line)
        if pattern.search(line):
            break


def generate_chunks(
    version: str, seed: str, corner1: tuple[int, int], corner2: tuple[int, int]
):
    x1 = min(corner1[0], corner2[0]) * 16
    z1 = min(corner1[1], corner2[1]) * 16
    x2 = (max(corner1[0], corner2[0]) + 1) * 16 - 1
    z2 = (max(corner1[1], corner2[1]) + 1) * 16 - 1
    log.info(
        "starting chunks generation",
        version=version,
        seed=seed,
        x1=x1,
        z1=z1,
        x2=x2,
        z2=z2,
    )
    PODMAN_API_URL = "tcp://localhost:8888"
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
            log.info("starting container")
            container = client.containers.run(  # pyright: ignore[reportAssignmentType]
                image="itzg/minecraft-server",
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
                detach=True,
                restart_policy={"Name": "unless-stopped"},
                stdout=True,
                stderr=True,
            )
            if not isinstance(container, Container):
                raise RuntimeError(
                    f"container has an incorrect type: {type(container)}"
                )
            log.info("container started", container_id=container.id)

            try:
                log.info("wait until server done")
                logs: Iterator[bytes] = container.logs(stream=True, follow=True)  # pyright: ignore[reportAssignmentType]
                wait_logs_until(logs, r'Done \(\d+\.?\d+s\)! For help, type "help"')

                rcon_commands = [
                    (
                        [
                            "rcon-cli",
                            "chunky",
                            "corners",
                            str(x1),
                            str(z1),
                            str(x2),
                            str(z2),
                        ],
                        None,
                    ),
                    (["rcon-cli", "chunky", "radius", "100"], None),
                    (["rcon-cli", "chunky", "start"], r"\[Chunky\] Task finished"),
                    (
                        ["rcon-cli", "save-all", "flush"],
                        r"ThreadedAnvilChunkStorage: All dimensions are saved",
                    ),
                ]
                for cmd, stop_pattern in rcon_commands:
                    log.info(
                        "running rcon command", command=cmd, stop_pattern=stop_pattern
                    )
                    (exit_code, output) = container.exec_run(
                        cmd, stdout=True, stderr=True, demux=True
                    )
                    if not isinstance(output, tuple):
                        raise RuntimeError(
                            f"container.exec_run has an incorrect return type: {output}"
                        )
                    if not isinstance(output[0], bytes | None) or not isinstance(
                        output[1], bytes | None
                    ):
                        raise RuntimeError(
                            f"container.exec_run has an incorrect return type: {output}"
                        )
                    log.debug(
                        "RCON command exited",
                        command=cmd,
                        exit_code=exit_code,
                        stdout=output[0],
                        stderr=output[1],
                    )
                    if exit_code != 0:
                        raise RuntimeError("RCON command failed")
                    if stop_pattern:
                        wait_logs_until(logs, stop_pattern)

                log.info("copying region file")
                dst = Path() / "temp" / (container.id or "unknown") / "region"
                dst.mkdir(parents=True, exist_ok=True)
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
                        container_id=container.id,
                        stderr=copy_result.stderr,
                    )
                    raise RuntimeError("Failed to copy region file")

            except Exception:
                log.exception("error in chunks generation")
                raise
            finally:
                log.info("stopping container", container_id=container.id)
                container.stop()
                container.remove()
                container = None

    except Exception:
        log.exception("error in chunks generation")
        raise
    finally:
        log.debug("terminating podman service")
        podman_service.terminate()
        podman_service.wait(timeout=30)
        log.info("chunks generation complete")


def prototype(
    version: str, seed: str, corner1: tuple[int, int], corner2: tuple[int, int]
):
    generate_chunks(version, seed, corner1, corner2)


def cli():
    typer.run(main)


if __name__ == "__main__":
    main()
