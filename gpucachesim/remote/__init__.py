import typing
import paramiko
import paramiko.channel
import scp
import re
import datetime
from io import BytesIO, StringIO
import tempfile
import sys
import time
from clear import clear
import os
import click
import select
from dotenv import load_dotenv
from pathlib import Path
import humanize
from wasabi import color
import socketserver as SocketServer

from gpucachesim.benchmarks import REPO_ROOT_DIR

DAS_ENV = REPO_ROOT_DIR / "das6.env"

DAS6_FORWARD_PORT = 2201
DAS5_FORWARD_PORT = 2202

SEC = 1
MIN = 60 * SEC
HOUR = 60 * MIN


@click.group()
def main():
    pass


class SSHClient:
    def __init__(self, host: str, username: str, password: str, port=22, timeout=60):
        self.host = host
        self.port = port
        self.username = username

        self.connection = paramiko.SSHClient()
        self.connection.load_system_host_keys()
        self.connection.set_missing_host_key_policy(paramiko.WarningPolicy())
        self.connection.connect(
            host,
            port,
            username=username,
            password=password,
            timeout=timeout,
        )

        # setup SCP
        self.scp_client = scp.SCPClient(self.connection.get_transport())

    def run_command(
        self, cmd: str
    ) -> typing.Tuple[int, paramiko.channel.ChannelFile, paramiko.channel.ChannelStderrFile]:
        _, stdout, stderr = self.connection.exec_command(cmd)
        stdout.channel.recv_exit_status()
        exit_status = stderr.channel.recv_exit_status()
        return exit_status, stdout, stderr

    def upload_data(
        self,
        data: typing.IO[typing.AnyStr],
        remote_path: os.PathLike,
    ):
        self.scp_client.putfo(data, remote_path=remote_path)
        print("uploaded data to {}:{}:{}".format(self.host, self.port, remote_path))

    def upload_file(
        self,
        local_path: os.PathLike,
        remote_path: os.PathLike,
        recursive: bool = False,
    ):
        self.scp_client.put(files=local_path, remote_path=remote_path, recursive=recursive)
        size = humanize.naturalsize(Path(local_path).stat().st_size, binary=True)
        print("uploaded {} ({}) to {}:{}:{}".format(local_path, size, self.host, self.port, remote_path))

    def file_exists(
        self,
        remote_path: os.PathLike,
    ):
        exit_status, _, _ = self.run_command("stat {}".format(remote_path))
        return exit_status == 0

    def upload_files_to_dir(
        self,
        local_paths: typing.Sequence[os.PathLike],
        remote_dir: os.PathLike,
    ):
        self.scp_client.put(files=local_paths, remote_path=remote_dir, recursive=True)
        print("uploaded {} files to {}:{}:{}".format(len(local_paths), self.host, self.port, remote_dir))

    def download_file(self, remote_path: os.PathLike, local_path: os.PathLike, recursive: bool = False):
        self.scp_client.get(remote_path=remote_path, local_path=local_path, recursive=recursive)
        size = humanize.naturalsize(Path(local_path).stat().st_size, binary=True)
        print("downloaded {}:{}:{} ({}) to {}".format(self.host, self.port, remote_path, size, local_path))

    def read_file_contents(self, remote_path: os.PathLike):
        with tempfile.NamedTemporaryFile() as temp_file:
            self.scp_client.get(remote_path=remote_path, local_path=temp_file.name)
            size = humanize.naturalsize(Path(temp_file.name).stat().st_size, binary=True)
            print("read file contents {}:{}:{} ({})".format(self.host, self.port, remote_path, size))
            return BytesIO(temp_file.read())

    def close(self):
        try:
            self.connection.close()
        except Exception as e:
            print(e)

        if self.scp_client:
            self.scp_client.close()


def duration_to_slurm(duration: datetime.timedelta):
    if duration.days > 0:
        raise NotImplementedError("durations of more than one day are not supported yet")
    hours, remainder = divmod(duration.seconds, 3600)
    minutes, seconds = divmod(remainder, 60)
    return "{:02}:{:02}:{:02}".format(int(hours), int(minutes), int(seconds))


class DAS6(SSHClient):
    def __init__(self, port=DAS6_FORWARD_PORT, timeout=60):
        load_dotenv(DAS_ENV)

        super().__init__(
            host="localhost",
            username=os.environ["DAS6_USERNAME"],
            password=os.environ["DAS6_PASSWORD"],
            port=port,
            timeout=timeout,
        )

        self.remote_scratch_dir = Path("/var/scratch") / self.username
        self.remote_pchase_executable = self.remote_scratch_dir / "pchase"
        self.remote_pchase_results_dir = self.remote_scratch_dir / "pchase-results"

    def run_pchase_sync(
        self,
        cmd,
        gpu,
        executable=None,
        force=False,
        timeout=4 * HOUR,
        retries=10,
    ) -> typing.Tuple[str, str]:
        executable = executable or self.remote_pchase_executable

        job_name = "-".join([Path(executable).name, str(gpu)] + cmd)
        remote_stdout_path = self.remote_pchase_results_dir / "{}.stdout".format(job_name)
        remote_stderr_path = self.remote_pchase_results_dir / "{}.stderr".format(job_name)

        print(job_name, cmd)

        # check if job already running
        running_job_names = self.get_running_job_names()
        if not force and job_name in running_job_names:
            raise ValueError("slurm job <{}> is already running".format(job_name))
        elif force:
            print(color("force re-running job {}".format(job_name), fg="red"))

        # check if results already exists
        if force or not self.file_exists(remote_stdout_path):
            job_id, _, _ = self.submit_pchase(gpu=gpu, name=job_name, args=cmd, timeout=timeout)
            print("submitted job <{}> [ID={}]".format(job_name, job_id))

            self.wait_for_job(job_id)
        else:
            print(
                color(
                    "re-using {} for job {}".format(remote_stdout_path, job_name),
                    fg="cyan",
                )
            )

        # copy stdout and stderr
        err = None
        for r in range(retries):
            if r > 0:
                print("reading stdout from {} (attempt {}/{})".format(remote_stdout_path, r + 1, retries))
            try:
                stdout = self.read_file_contents(remote_path=remote_stdout_path).read().decode("utf-8")
                stderr = self.read_file_contents(remote_path=remote_stderr_path).read().decode("utf-8")
                if stdout.strip() != "":
                    return stdout, stderr
            except Exception as e:
                print("reading stdout from {} failed: {}".format(remote_stdout_path, e))
                err = e
            time.sleep(5 * SEC)

        raise err or ValueError("stdout is empty")

    def submit_pchase(
        self,
        gpu,
        args,
        name=None,
        executable=None,
        timeout=4 * HOUR,
    ) -> typing.Tuple[typing.Optional[int], str, typing.Tuple[os.PathLike, os.PathLike]]:
        # upload pchase executable
        # client.upload_file(local_path=local_pchase_executable, remote_path=remote_pchase_executable)

        executable = executable or self.remote_pchase_executable

        # load cuda toolkit
        exit_status, stdout, stderr = self.run_command("module load cuda11.7/toolkit")
        print(stderr.read().decode("utf-8"), end="")
        print(stdout.read().decode("utf-8"), end="")
        assert exit_status == 0

        # create results dir
        exit_status, stdout, stderr = self.run_command("mkdir -p {}".format(self.remote_pchase_results_dir))
        print(stderr.read().decode("utf-8"), end="")
        print(stdout.read().decode("utf-8"), end="")
        assert exit_status == 0

        # build slurm script
        job_name = name or "-".join([Path(executable).name, str(gpu)] + args)
        remote_slurm_job_path = self.remote_pchase_results_dir / f"{job_name}.job"
        remote_slurm_stdout_path = self.remote_pchase_results_dir / f"{job_name}.stdout"
        remote_slurm_stderr_path = self.remote_pchase_results_dir / f"{job_name}.stderr"

        slurm_script = "#!/bin/sh\n"
        slurm_script += "#SBATCH --job-name={}\n".format(job_name)
        slurm_script += "#SBATCH --output={}\n".format(str(remote_slurm_stdout_path))
        slurm_script += "#SBATCH --error={}\n".format(str(remote_slurm_stderr_path))
        if isinstance(timeout, int):
            slurm_script += "#SBATCH --time={}\n".format(duration_to_slurm(datetime.timedelta(seconds=timeout)))
        if isinstance(timeout, datetime.timedelta):
            slurm_script += "#SBATCH --time={}\n".format(duration_to_slurm(timeout))
        slurm_script += "#SBATCH -N 1\n"
        slurm_script += "#SBATCH -C {}\n".format(gpu)
        slurm_script += "#SBATCH --gres=gpu:1\n"
        slurm_script += "{} {}\n".format(executable, " ".join(args))

        # upload slurm script
        self.upload_data(data=StringIO(slurm_script), remote_path=remote_slurm_job_path)

        # enqueue slurm job
        exit_status, stdout, stderr = self.run_command("sbatch {}".format(remote_slurm_job_path))
        print(stderr.read().decode("utf-8"), end="")

        stdout = stdout.read().decode("utf-8")
        print(stdout, end="")
        assert exit_status == 0

        job_id = extract_slurm_job_id(stdout)

        return job_id, job_name, (remote_slurm_stdout_path, remote_slurm_stderr_path)

    def get_running_job_names(self):
        exit_status, stdout, stderr = self.run_command(
            'squeue --user {} --format="%j" -t RUNNING'.format(self.username)
        )
        stderr = stderr.read().decode("utf-8")
        print(stderr, end="")
        assert exit_status == 0
        job_names = sorted([line.strip() for line in stdout.readlines()])
        return job_names[1:]

    def get_running_job_ids(self):
        exit_status, stdout, stderr = self.run_command(
            'squeue --user {} --format="%i" -t RUNNING'.format(self.username)
        )
        stderr = stderr.read().decode("utf-8")
        print(stderr, end="")
        assert exit_status == 0
        job_ids = [line.strip() for line in stdout.readlines()]
        job_ids = sorted([int(job_id) for job_id in job_ids[1:]])
        return job_ids

    def wait_for_job(self, job_id, interval_sec=5.0):
        print("waiting for job {} to complete".format(job_id))
        while True:
            job_ids = self.get_running_job_ids()
            if job_id not in job_ids:
                print("job {} completed".format(job_id))
                break
            print("running jobs: {}".format(job_ids))
            time.sleep(interval_sec)

    def print_squeue(self, user=None):
        cmd = ["squeue"]
        if user is not None:
            cmd += ["--user", str(user)]
        exit_status, stdout, stderr = self.run_command(" ".join(cmd))
        clear()
        for line in stderr.readlines():
            print(line.strip())
        for line in stdout.readlines():
            print(line.strip())
        assert exit_status == 0


def extract_slurm_job_id(stdout: str) -> typing.Optional[int]:
    match = re.match(r"Submitted batch job (\d+)", stdout)
    if match is not None:
        return int(match.group(1))
    return None


class ForwardServer(SocketServer.ThreadingTCPServer):
    daemon_threads = True
    allow_reuse_address = True


def format_address(addr: typing.Tuple[str, int]) -> str:
    return "{}:{}".format(addr[0], addr[1])


class Handler(SocketServer.BaseRequestHandler):
    ssh_transport: paramiko.Transport
    ssh_remote_host: str
    ssh_remote_port: int

    def handle(self):
        try:
            print("forward: opening new channel to {}:{}".format(self.ssh_remote_host, self.ssh_remote_port))
            channel = self.ssh_transport.open_channel(
                kind="direct-tcpip",
                dest_addr=(self.ssh_remote_host, self.ssh_remote_port),
                src_addr=self.request.getpeername(),
                timeout=60,
            )
        except Exception as e:
            print("incoming request to {}:{} failed: {}".format(self.ssh_remote_host, self.ssh_remote_port, repr(e)))
            raise e

        if channel is None:
            raise ValueError(
                "incoming request to {}:{} was rejected by the SSH server".format(
                    self.ssh_remote_host, self.ssh_remote_port
                )
            )

        print(
            "opened tunnel: {} -> {} -> {}".format(
                format_address(self.request.getpeername()),
                format_address(channel.getpeername()),
                format_address((self.ssh_remote_host, self.ssh_remote_port)),
            )
        )
        while True:
            r, _, _ = select.select([self.request, channel], [], [])
            if self.request in r:
                data = self.request.recv(1024)
                if len(data) == 0:
                    break
                channel.send(data)
            if channel in r:
                data = channel.recv(1024)
                if len(data) == 0:
                    break
                self.request.send(data)

        peername = self.request.getpeername()
        channel.close()
        self.request.close()
        print("tunnel {} closed".format(format_address(peername)))


def forward_tunnel(transport, local_port: int, remote_host: str, remote_port=22):
    class SubHander(Handler):
        ssh_remote_host = remote_host
        ssh_remote_port = remote_port
        ssh_transport = transport

    ForwardServer(("", local_port), SubHander).serve_forever()


def tunnel_das(das_host, local_port):
    vu_host = os.environ["VU_HOST"]
    vu_username = os.environ["VU_USERNAME"]
    vu_password = os.environ["VU_PASSWORD"]

    client = None
    try:
        client = SSHClient(host=vu_host, username=vu_username, password=vu_password)
        print("connected to VU")
        forward_tunnel(
            client.connection.get_transport(),
            local_port=local_port,
            remote_host=das_host,
        )
        client.close()
    except Exception as e:
        if client is not None:
            client.close()
        raise e


@main.command()
@click.option(
    "--port",
    "local_port",
    type=int,
    default=DAS5_FORWARD_PORT,
    help="Local ssh forwarding port",
)
def tunnel_das5(local_port):
    tunnel_das(das_host=os.environ["DAS5_HOST"], local_port=local_port)


@main.command()
@click.option(
    "--port",
    "local_port",
    type=int,
    default=DAS6_FORWARD_PORT,
    help="Local ssh forwarding port",
)
def tunnel_das6(local_port):
    tunnel_das(das_host=os.environ["DAS6_HOST"], local_port=local_port)


@main.command()
@click.option(
    "--port",
    "local_port",
    type=int,
    default=DAS6_FORWARD_PORT,
    help="Local ssh forwarding port",
)
def squeue(local_port):
    das6 = None
    try:
        das6 = DAS6(port=local_port)
        print("connected to DAS6")

        while True:
            das6.print_squeue(user=das6.username)
            time.sleep(2.0)

    except Exception as e:
        if das6 is not None:
            das6.close()
        raise e


@main.command(
    context_settings=dict(
        ignore_unknown_options=True,
        allow_extra_args=True,
    )
)
@click.option(
    "--port",
    "local_port",
    type=int,
    default=DAS6_FORWARD_PORT,
    help="Local ssh forwarding port",
)
@click.option(
    "--gpu",
    "gpu",
    type=str,
    default="A4000",
    help="Default GPU device",
)
@click.pass_context
def submit_pchase(ctx, local_port, gpu):
    args = list(ctx.args)
    das6 = None
    try:
        das6 = DAS6(port=local_port)
        print("connected to DAS6")
        job_id, job_name, _ = das6.submit_pchase(gpu=gpu, args=args)
        print("submitted job <{}> [ID={}]".format(job_name, job_id))
        das6.close()
    except Exception as e:
        if das6 is not None:
            das6.close()
        raise e


if __name__ == "__main__":
    load_dotenv(DAS_ENV)
    main()