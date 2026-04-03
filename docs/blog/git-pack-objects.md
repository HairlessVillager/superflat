# git pack-objects 实现分析

`git pack-objects` 是 Git 中用于创建打包文件的核心命令，其主要实现在 `builtin/pack-objects.c` 文件中。

## 核心实现架构

### 主入口函数

`cmd_pack_objects()` 是命令的主入口点，负责参数解析和整体流程控制 [1](#0-0) 。该函数处理多种选项，包括压缩级别、增量窗口大小、线程数等参数。

### 对象枚举阶段

实现支持多种对象枚举模式：

1. **标准模式**：从标准输入读取对象列表
2. **修订模式**：使用 `--revs` 选项，通过 `get_object_list()` 进行对象遍历 [2](#0-1)
3. **包文件模式**：使用 `--stdin-packs` 从现有包文件中读取对象 [3](#0-2)
4. **废弃对象模式**：使用 `--cruft` 打包不可达对象 [4](#0-3)

### 包准备阶段

在 `prepare_pack()` 函数中进行增量压缩计算 [5](#0-4) 。该阶段：

- 对象按类型、大小和名称哈希排序
- 在指定的窗口大小内寻找最佳增量压缩
- 限制增量链深度以平衡压缩率和性能

### 包文件写入阶段

`write_pack_file()` 函数负责实际的包文件生成 [6](#0-5) ，包括：

- 写入包文件头部（签名、版本号、对象数量）
- 写入压缩后的对象数据
- 生成对应的索引文件

## 性能优化特性

实现包含多项性能优化：

- **位图索引**：使用 `--use-bitmap-index` 加速对象计数 [9](#0-8)
- **稀疏算法**：使用 `--sparse` 只遍历引入新对象的路径 [10](#0-9)
- **多线程压缩**：通过 `--threads` 参数并行处理增量搜索 [11](#0-10)
- **增量重用**：重用现有增量以避免重复计算 [12](#0-11)

## 配置和扩展

`prepare_pack_objects()` 函数展示了如何配置 pack-objects 的各种参数 [13](#0-12) ，这些参数包括窗口大小、内存限制、增量深度等，允许根据不同场景优化打包性能。

---

## Notes

- `git pack-objects` 的实现高度模块化，支持多种使用场景
- 网络传输（push/fetch）和本地打包（gc/repack）都使用相同的核心实现
- 包文件格式遵循 `Documentation/gitformat-pack.adoc` 中定义的规范 [14](#0-13)
- 实现中包含了丰富的性能优化选项，可根据仓库特点调整参数

Wiki pages you might want to explore:

- [Key Operations (git/git)](/wiki/git/git#3)

### Citations

**File:** builtin/pack-objects.c (L4864-4967)

```c
int cmd_pack_objects(int argc,
		     const char **argv,
		     const char *prefix,
		     struct repository *repo UNUSED)
{
	int use_internal_rev_list = 0;
	int all_progress_implied = 0;
	struct strvec rp = STRVEC_INIT;
	int rev_list_unpacked = 0, rev_list_all = 0, rev_list_reflog = 0;
	int rev_list_index = 0;
	enum stdin_packs_mode stdin_packs = STDIN_PACKS_MODE_NONE;
	struct string_list keep_pack_list = STRING_LIST_INIT_NODUP;
	struct list_objects_filter_options filter_options =
		LIST_OBJECTS_FILTER_INIT;

	struct option pack_objects_options[] = {
		OPT_CALLBACK_F('q', "quiet", &progress, NULL,
			       N_("do not show progress meter"),
			       PARSE_OPT_NOARG, option_parse_quiet),
		OPT_SET_INT(0, "progress", &progress,
			    N_("show progress meter"), 1),
		OPT_SET_INT(0, "all-progress", &progress,
			    N_("show progress meter during object writing phase"), 2),
		OPT_BOOL(0, "all-progress-implied",
			 &all_progress_implied,
			 N_("similar to --all-progress when progress meter is shown")),
		OPT_CALLBACK_F(0, "index-version", &pack_idx_opts, N_("<version>[,<offset>]"),
		  N_("write the pack index file in the specified idx format version"),
		  PARSE_OPT_NONEG, option_parse_index_version),
		OPT_UNSIGNED(0, "max-pack-size", &pack_size_limit,
			     N_("maximum size of each output pack file")),
		OPT_BOOL(0, "local", &local,
			 N_("ignore borrowed objects from alternate object store")),
		OPT_BOOL(0, "incremental", &incremental,
			 N_("ignore packed objects")),
		OPT_INTEGER(0, "window", &window,
			    N_("limit pack window by objects")),
		OPT_UNSIGNED(0, "window-memory", &window_memory_limit,
			     N_("limit pack window by memory in addition to object limit")),
		OPT_INTEGER(0, "depth", &depth,
			    N_("maximum length of delta chain allowed in the resulting pack")),
		OPT_BOOL(0, "reuse-delta", &reuse_delta,
			 N_("reuse existing deltas")),
		OPT_BOOL(0, "reuse-object", &reuse_object,
			 N_("reuse existing objects")),
		OPT_BOOL(0, "delta-base-offset", &allow_ofs_delta,
			 N_("use OFS_DELTA objects")),
		OPT_INTEGER(0, "threads", &delta_search_threads,
			    N_("use threads when searching for best delta matches")),
		OPT_BOOL(0, "non-empty", &non_empty,
			 N_("do not create an empty pack output")),
		OPT_BOOL(0, "revs", &use_internal_rev_list,
			 N_("read revision arguments from standard input")),
		OPT_SET_INT_F(0, "unpacked", &rev_list_unpacked,
			      N_("limit the objects to those that are not yet packed"),
			      1, PARSE_OPT_NONEG),
		OPT_SET_INT_F(0, "all", &rev_list_all,
			      N_("include objects reachable from any reference"),
			      1, PARSE_OPT_NONEG),
		OPT_SET_INT_F(0, "reflog", &rev_list_reflog,
			      N_("include objects referred by reflog entries"),
			      1, PARSE_OPT_NONEG),
		OPT_SET_INT_F(0, "indexed-objects", &rev_list_index,
			      N_("include objects referred to by the index"),
			      1, PARSE_OPT_NONEG),
		OPT_CALLBACK_F(0, "stdin-packs", &stdin_packs, N_("mode"),
			     N_("read packs from stdin"),
			     PARSE_OPT_OPTARG, parse_stdin_packs_mode),
		OPT_BOOL(0, "stdin-packs", &stdin_packs,
			 N_("read packs from stdin")),
		OPT_BOOL(0, "stdout", &pack_to_stdout,
			 N_("output pack to stdout")),
		OPT_BOOL(0, "include-tag", &include_tag,
			 N_("include tag objects that refer to objects to be packed")),
		OPT_BOOL(0, "keep-unreachable", &keep_unreachable,
			 N_("keep unreachable objects")),
		OPT_BOOL(0, "pack-loose-unreachable", &pack_loose_unreachable,
			 N_("pack loose unreachable objects")),
		OPT_CALLBACK_F(0, "unpack-unreachable", NULL, N_("time"),
		  N_("unpack unreachable objects newer than <time>"),
		  PARSE_OPT_OPTARG, option_parse_unpack_unreachable),
		OPT_BOOL(0, "cruft", &cruft, N_("create a cruft pack")),
		OPT_CALLBACK_F(0, "cruft-expiration", NULL, N_("time"),
		  N_("expire cruft objects older than <time>"),
		  PARSE_OPT_OPTARG, option_parse_cruft_expiration),
		OPT_BOOL(0, "sparse", &sparse,
			 N_("use the sparse reachability algorithm")),
		OPT_BOOL(0, "thin", &thin,
			 N_("create thin packs")),
		OPT_BOOL(0, "path-walk", &path_walk,
			 N_("use the path-walk API to walk objects when possible")),
		OPT_BOOL(0, "shallow", &shallow,
			 N_("create packs suitable for shallow fetches")),
		OPT_BOOL(0, "honor-pack-keep", &ignore_packed_keep_on_disk,
			 N_("ignore packs that have companion .keep file")),
		OPT_STRING_LIST(0, "keep-pack", &keep_pack_list, N_("name"),
				N_("ignore this pack")),
		OPT_INTEGER(0, "compression", &pack_compression_level,
			    N_("pack compression level")),
		OPT_BOOL(0, "keep-true-parents", &grafts_keep_true_parents,
			 N_("do not hide commits by grafts")),
		OPT_BOOL(0, "use-bitmap-index", &use_bitmap_index,
			 N_("use a bitmap index if available to speed up counting objects")),
		OPT_SET_INT(0, "write-bitmap-index", &write_bitmap_index,
```

**File:** builtin/pack-objects.c (L5227-5230)

```c
	if (stdin_packs) {
		read_stdin_packs(stdin_packs, rev_list_unpacked);
	} else if (cruft) {
		read_cruft_objects();
```

**File:** builtin/pack-objects.c (L5231-5244)

```c
	} else if (!use_internal_rev_list) {
		read_object_list_from_stdin();
	} else {
		struct rev_info revs;

		repo_init_revisions(the_repository, &revs, NULL);
		list_objects_filter_copy(&revs.filter, &filter_options);
		if (exclude_promisor_objects_best_effort) {
			revs.include_check = is_not_in_promisor_pack;
			revs.include_check_obj = is_not_in_promisor_pack_obj;
		}
		get_object_list(&revs, &rp);
		release_revisions(&revs);
	}
```

**File:** builtin/pack-objects.c (L5258-5258)

```c
		prepare_pack(window, depth);
```

**File:** builtin/pack-objects.c (L5265-5265)

```c
	write_pack_file();
```

**File:** send-pack.c (L60-151)

```c
static int pack_objects(struct repository *r,
			int fd, struct ref *refs, struct oid_array *advertised,
			struct oid_array *negotiated,
			struct send_pack_args *args)
{
	/*
	 * The child becomes pack-objects --revs; we feed
	 * the revision parameters to it via its stdin and
	 * let its stdout go back to the other end.
	 */
	struct child_process po = CHILD_PROCESS_INIT;
	FILE *po_in;
	int rc;

	trace2_region_enter("send_pack", "pack_objects", r);
	strvec_push(&po.args, "pack-objects");
	strvec_push(&po.args, "--all-progress-implied");
	strvec_push(&po.args, "--revs");
	strvec_push(&po.args, "--stdout");
	if (args->use_thin_pack)
		strvec_push(&po.args, "--thin");
	if (args->use_ofs_delta)
		strvec_push(&po.args, "--delta-base-offset");
	if (args->quiet || !args->progress)
		strvec_push(&po.args, "-q");
	if (args->progress)
		strvec_push(&po.args, "--progress");
	if (is_repository_shallow(r))
		strvec_push(&po.args, "--shallow");
	if (args->disable_bitmaps)
		strvec_push(&po.args, "--no-use-bitmap-index");
	po.in = -1;
	po.out = args->stateless_rpc ? -1 : fd;
	po.git_cmd = 1;
	po.clean_on_exit = 1;
	if (start_command(&po))
		die_errno("git pack-objects failed");

	/*
	 * We feed the pack-objects we just spawned with revision
	 * parameters by writing to the pipe.
	 */
	po_in = xfdopen(po.in, "w");
	for (size_t i = 0; i < advertised->nr; i++)
		feed_object(r, &advertised->oid[i], po_in, 1);
	for (size_t i = 0; i < negotiated->nr; i++)
		feed_object(r, &negotiated->oid[i], po_in, 1);

	while (refs) {
		if (!is_null_oid(&refs->old_oid))
			feed_object(r, &refs->old_oid, po_in, 1);
		if (!is_null_oid(&refs->new_oid))
			feed_object(r, &refs->new_oid, po_in, 0);
		refs = refs->next;
	}

	fflush(po_in);
	if (ferror(po_in))
		die_errno("error writing to pack-objects");
	fclose(po_in);

	if (args->stateless_rpc) {
		char *buf = xmalloc(LARGE_PACKET_MAX);
		while (1) {
			ssize_t n = xread(po.out, buf, LARGE_PACKET_MAX);
			if (n <= 0)
				break;
			send_sideband(fd, -1, buf, n, LARGE_PACKET_MAX);
		}
		free(buf);
		close(po.out);
		po.out = -1;
	}

	rc = finish_command(&po);
	if (rc) {
		/*
		 * For a normal non-zero exit, we assume pack-objects wrote
		 * something useful to stderr. For death by signal, though,
		 * we should mention it to the user. The exception is SIGPIPE
		 * (141), because that's a normal occurrence if the remote end
		 * hangs up (and we'll report that by trying to read the unpack
		 * status).
		 */
		if (rc > 128 && rc != 141)
			error("pack-objects died of signal %d", rc - 128);
		trace2_region_leave("send_pack", "pack_objects", r);
		return -1;
	}
	trace2_region_leave("send_pack", "pack_objects", r);
	return 0;
}
```

**File:** upload-pack.c (L285-366)

```c
static void create_pack_file(struct upload_pack_data *pack_data,
			     const struct string_list *uri_protocols)
{
	struct child_process pack_objects = CHILD_PROCESS_INIT;
	struct output_state *output_state = xcalloc(1, sizeof(struct output_state));
	char progress[128];
	char abort_msg[] = "aborting due to possible repository "
		"corruption on the remote side.";
	ssize_t sz;
	int i;
	FILE *pipe_fd;

	if (!pack_data->pack_objects_hook)
		pack_objects.git_cmd = 1;
	else {
		strvec_push(&pack_objects.args, pack_data->pack_objects_hook);
		strvec_push(&pack_objects.args, "git");
		pack_objects.use_shell = 1;
	}

	if (pack_data->shallow_nr) {
		strvec_push(&pack_objects.args, "--shallow-file");
		strvec_push(&pack_objects.args, "");
	}
	strvec_push(&pack_objects.args, "pack-objects");
	strvec_push(&pack_objects.args, "--revs");
	if (pack_data->use_thin_pack)
		strvec_push(&pack_objects.args, "--thin");

	strvec_push(&pack_objects.args, "--stdout");
	if (pack_data->shallow_nr)
		strvec_push(&pack_objects.args, "--shallow");
	if (!pack_data->no_progress)
		strvec_push(&pack_objects.args, "--progress");
	if (pack_data->use_ofs_delta)
		strvec_push(&pack_objects.args, "--delta-base-offset");
	if (pack_data->use_include_tag)
		strvec_push(&pack_objects.args, "--include-tag");
	if (repo_has_accepted_promisor_remote(the_repository))
		strvec_push(&pack_objects.args, "--missing=allow-promisor");
	if (pack_data->filter_options.choice) {
		const char *spec =
			expand_list_objects_filter_spec(&pack_data->filter_options);
		strvec_pushf(&pack_objects.args, "--filter=%s", spec);
	}
	if (uri_protocols) {
		for (i = 0; i < uri_protocols->nr; i++)
			strvec_pushf(&pack_objects.args, "--uri-protocol=%s",
					 uri_protocols->items[i].string);
	}

	pack_objects.in = -1;
	pack_objects.out = -1;
	pack_objects.err = -1;
	pack_objects.clean_on_exit = 1;

	if (start_command(&pack_objects))
		die("git upload-pack: unable to fork git-pack-objects");

	pipe_fd = xfdopen(pack_objects.in, "w");

	if (pack_data->shallow_nr)
		for_each_commit_graft(write_one_shallow, pipe_fd);

	for (i = 0; i < pack_data->want_obj.nr; i++)
		fprintf(pipe_fd, "%s\n",
			oid_to_hex(&pack_data->want_obj.objects[i].item->oid));
	fprintf(pipe_fd, "--not\n");
	for (i = 0; i < pack_data->have_obj.nr; i++)
		fprintf(pipe_fd, "%s\n",
			oid_to_hex(&pack_data->have_obj.objects[i].item->oid));
	for (i = 0; i < pack_data->extra_edge_obj.nr; i++)
		fprintf(pipe_fd, "%s\n",
			oid_to_hex(&pack_data->extra_edge_obj.objects[i].item->oid));
	fprintf(pipe_fd, "\n");
	fflush(pipe_fd);
	fclose(pipe_fd);

	/* We read from pack_objects.err to capture stderr output for
	 * progress bar, and pack_objects.out to capture the pack data.
	 */

```

**File:** Documentation/git-pack-objects.adoc (L246-256)

```text
--sparse::
--no-sparse::
	Toggle the "sparse" algorithm to determine which objects to include in
	the pack, when combined with the "--revs" option. This algorithm
	only walks trees that appear in paths that introduce new objects.
	This can have significant performance benefits when computing
	a pack to send a small change. However, it is possible that extra
	objects are added to the pack-file if the included commits contain
	certain types of direct renames. If this option is not included,
	it defaults to the value of `pack.useSparse`, which is true unless
	otherwise specified.
```

**File:** Documentation/git-pack-objects.adoc (L289-297)

```text
--threads=<n>::
	Specifies the number of threads to spawn when searching for best
	delta matches.  This requires that pack-objects be compiled with
	pthreads otherwise this option is ignored with a warning.
	This is meant to reduce packing time on multiprocessor machines.
	The required amount of memory for the delta search window is
	however multiplied by the number of threads.
	Specifying 0 will cause Git to auto-detect the number of CPU's
	and set the number of threads accordingly.
```

**File:** repack.c (L12-46)

```c
void prepare_pack_objects(struct child_process *cmd,
			  const struct pack_objects_args *args,
			  const char *out)
{
	strvec_push(&cmd->args, "pack-objects");
	if (args->window)
		strvec_pushf(&cmd->args, "--window=%s", args->window);
	if (args->window_memory)
		strvec_pushf(&cmd->args, "--window-memory=%s", args->window_memory);
	if (args->depth)
		strvec_pushf(&cmd->args, "--depth=%s", args->depth);
	if (args->threads)
		strvec_pushf(&cmd->args, "--threads=%s", args->threads);
	if (args->max_pack_size)
		strvec_pushf(&cmd->args, "--max-pack-size=%lu", args->max_pack_size);
	if (args->no_reuse_delta)
		strvec_pushf(&cmd->args, "--no-reuse-delta");
	if (args->no_reuse_object)
		strvec_pushf(&cmd->args, "--no-reuse-object");
	if (args->name_hash_version)
		strvec_pushf(&cmd->args, "--name-hash-version=%d", args->name_hash_version);
	if (args->path_walk)
		strvec_pushf(&cmd->args, "--path-walk");
	if (args->local)
		strvec_push(&cmd->args,  "--local");
	if (args->quiet)
		strvec_push(&cmd->args,  "--quiet");
	if (args->delta_base_offset)
		strvec_push(&cmd->args,  "--delta-base-offset");
	if (!args->pack_kept_objects)
		strvec_push(&cmd->args,  "--honor-pack-keep");
	strvec_push(&cmd->args, out);
	cmd->git_cmd = 1;
	cmd->out = -1;
}
```

**File:** Documentation/gitformat-pack.adoc (L39-60)

```text
== pack-*.pack files have the following format:

   - A header appears at the beginning and consists of the following:

     4-byte signature:
         The signature is: {'P', 'A', 'C', 'K'}

     4-byte version number (network byte order):
	 Git currently accepts version number 2 or 3 but
         generates version 2 only.

     4-byte number of objects contained in the pack (network byte order)

     Observation: we cannot have more than 4G versions ;-) and
     more than 4G objects in a pack.

   - The header is followed by a number of object entries, each of
     which looks like this:

     (undeltified representation)
     n-byte type and length (3-bit type, (n-1)*7+4-bit length)
     compressed data
```

## builtin/pack-objects.c

### find_deltas

# find_deltas 运行机制

`find_deltas` 是 Git pack-objects 中负责增量压缩的核心函数，它使用滑动窗口算法在对象间寻找最佳的增量压缩关系。

## 核心算法流程

### 1. 滑动窗口机制

`find_deltas` 使用一个固定大小的滑动窗口来寻找增量压缩机会 [1](#2-0) ：

```c
static void find_deltas(struct object_entry **list, unsigned *list_size,
                        int window, int depth, unsigned *processed)
{
    uint32_t i, idx = 0, count = 0;
    struct unpacked *array;
    unsigned long mem_usage = 0;

    CALLOC_ARRAY(array, window);

    for (;;) {
        // 处理当前对象
        // 在窗口内寻找最佳增量基础
        // 管理内存使用
    }
}
```

### 2. 对象排序策略

在调用 `find_deltas` 之前，对象会通过 `type_size_sort` 函数排序 [2](#2-1) ，排序优先级为：

1. **对象类型**：相同类型的对象优先配对
2. **路径哈希**：相似路径的对象优先
3. **首选基础**：thin pack 中的基础对象优先
4. **对象大小**：大对象到小对象的增量压缩

### 3. 增量尝试过程

对于每个对象，`find_deltas` 在窗口内向后搜索可能的增量基础 [3](#2-2) ：

```c
j = window;
while (--j > 0) {
    int ret;
    uint32_t other_idx = idx + j;
    struct unpacked *m;
    if (other_idx >= window)
        other_idx -= window;
    m = array + other_idx;
    if (!m->entry)
        break;
    ret = try_delta(n, m, max_depth, &mem_usage);
    if (ret < 0)
        break;
    else if (ret > 0)
        best_base = other_idx;
}
```

### 4. try_delta 函数

`try_delta` 函数实际执行增量压缩尝试 [4](#2-3) ：

1. **创建增量索引**：为源对象创建增量索引
2. **生成增量数据**：使用索引计算增量
3. **验证增量效果**：确保增量比原始对象更小
4. **缓存管理**：决定是否缓存增量数据

### 5. 内存管理

`find_deltas` 严格控制内存使用 [5](#2-4) ：

```c
while (window_memory_limit &&
       mem_usage > window_memory_limit &&
       count > 1) {
    const uint32_t tail = (idx + window - count) % window;
    mem_usage -= free_unpacked(array + tail);
    count--;
}
```

## 多线程支持

### 1. 线程化搜索

`ll_find_deltas` 函数实现了多线程版本的增量搜索 [6](#2-5) ：

- 将对象列表分割给多个线程
- 实现工作窃取机制进行负载均衡
- 使用条件变量同步线程状态

### 2. Path-Walk 模式

当启用 `--path-walk` 时，使用 `ll_find_deltas_by_region` 按区域处理增量压缩 [7](#2-6) 。

## 调用链路

### 1. 入口点

`prepare_pack` 函数是增量压缩的入口点 [8](#2-7) ：

```c
static void prepare_pack(int window, int depth)
{
    // 获取对象详情
    get_object_details();

    // 根据模式选择增量搜索方式
    if (path_walk)
        ll_find_deltas_by_region(to_pack.objects, to_pack.regions,
                                 0, to_pack.nr_regions);

    // 传统模式
    if (nr_deltas && n > 1) {
        QSORT(delta_list, n, type_size_sort);
        ll_find_deltas(delta_list, n, window+1, depth, &nr_done);
    }
}
```

### 2. 对象筛选

只有满足条件的对象才会参与增量压缩 [9](#2-8) ：

- 不是已有增量
- 类型有效且大小足够（>50字节）
- 未标记为 `no_try_delta`

## 增量链管理

### 1. 深度限制

`find_deltas` 会检查增量链深度，避免过深的增量链 [10](#2-9) ：

```c
max_depth = depth;
if (DELTA_CHILD(entry)) {
    max_depth -= check_delta_limit(entry, 0);
    if (max_depth <= 0)
        goto next;
}
```

### 2. 基础对象优化

找到增量后，会将最佳基础对象移到窗口前面，延长其保留时间 [11](#2-10) 。

## Notes

- `find_deltas` 是 Git 增量压缩的核心，通过滑动窗口算法平衡压缩效果和性能
- 算法优先考虑从大对象到小对象的增量，以获得更好的压缩率
- 内存管理是关键，通过 `window_memory_limit` 控制峰值内存使用
- 多线程支持通过工作分割和窃取实现高效并行处理
- Path-Walk 模式提供了按路径组织的替代增量搜索策略

### Citations

**File:** builtin/pack-objects.c (L2604-2635)

```c
static int type_size_sort(const void *_a, const void *_b)
{
	const struct object_entry *a = *(struct object_entry **)_a;
	const struct object_entry *b = *(struct object_entry **)_b;
	const enum object_type a_type = oe_type(a);
	const enum object_type b_type = oe_type(b);
	const unsigned long a_size = SIZE(a);
	const unsigned long b_size = SIZE(b);

	if (a_type > b_type)
		return -1;
	if (a_type < b_type)
		return 1;
	if (a->hash > b->hash)
		return -1;
	if (a->hash < b->hash)
		return 1;
	if (a->preferred_base > b->preferred_base)
		return -1;
	if (a->preferred_base < b->preferred_base)
		return 1;
	if (use_delta_islands) {
		const int island_cmp = island_delta_cmp(&a->idx.oid, &b->idx.oid);
		if (island_cmp)
			return island_cmp;
	}
	if (a_size > b_size)
		return -1;
	if (a_size < b_size)
		return 1;
	return a < b ? -1 : (a > b);  /* newest first */
}
```

**File:** builtin/pack-objects.c (L2842-2891)

```c
	if (!src->index) {
		src->index = create_delta_index(src->data, src_size);
		if (!src->index) {
			static int warned = 0;
			if (!warned++)
				warning(_("suboptimal pack - out of memory"));
			return 0;
		}
		*mem_usage += sizeof_delta_index(src->index);
	}

	delta_buf = create_delta(src->index, trg->data, trg_size, &delta_size, max_size);
	if (!delta_buf)
		return 0;

	if (DELTA(trg_entry)) {
		/* Prefer only shallower same-sized deltas. */
		if (delta_size == DELTA_SIZE(trg_entry) &&
		    src->depth + 1 >= trg->depth) {
			free(delta_buf);
			return 0;
		}
	}

	/*
	 * Handle memory allocation outside of the cache
	 * accounting lock.  Compiler will optimize the strangeness
	 * away when NO_PTHREADS is defined.
	 */
	free(trg_entry->delta_data);
	cache_lock();
	if (trg_entry->delta_data) {
		delta_cache_size -= DELTA_SIZE(trg_entry);
		trg_entry->delta_data = NULL;
	}
	if (delta_cacheable(src_size, trg_size, delta_size)) {
		delta_cache_size += delta_size;
		cache_unlock();
		trg_entry->delta_data = xrealloc(delta_buf, delta_size);
	} else {
		cache_unlock();
		free(delta_buf);
	}

	SET_DELTA(trg_entry, src_entry);
	SET_DELTA_SIZE(trg_entry, delta_size);
	trg->depth = src->depth + 1;

	return 1;
}
```

**File:** builtin/pack-objects.c (L2920-2991)

```c
static void find_deltas(struct object_entry **list, unsigned *list_size,
			int window, int depth, unsigned *processed)
{
	uint32_t i, idx = 0, count = 0;
	struct unpacked *array;
	unsigned long mem_usage = 0;

	CALLOC_ARRAY(array, window);

	for (;;) {
		struct object_entry *entry;
		struct unpacked *n = array + idx;
		int j, max_depth, best_base = -1;

		progress_lock();
		if (!*list_size) {
			progress_unlock();
			break;
		}
		entry = *list++;
		(*list_size)--;
		if (!entry->preferred_base) {
			(*processed)++;
			display_progress(progress_state, *processed);
		}
		progress_unlock();

		mem_usage -= free_unpacked(n);
		n->entry = entry;

		while (window_memory_limit &&
		       mem_usage > window_memory_limit &&
		       count > 1) {
			const uint32_t tail = (idx + window - count) % window;
			mem_usage -= free_unpacked(array + tail);
			count--;
		}

		/* We do not compute delta to *create* objects we are not
		 * going to pack.
		 */
		if (entry->preferred_base)
			goto next;

		/*
		 * If the current object is at pack edge, take the depth the
		 * objects that depend on the current object into account
		 * otherwise they would become too deep.
		 */
		max_depth = depth;
		if (DELTA_CHILD(entry)) {
			max_depth -= check_delta_limit(entry, 0);
			if (max_depth <= 0)
				goto next;
		}

		j = window;
		while (--j > 0) {
			int ret;
			uint32_t other_idx = idx + j;
			struct unpacked *m;
			if (other_idx >= window)
				other_idx -= window;
			m = array + other_idx;
			if (!m->entry)
				break;
			ret = try_delta(n, m, max_depth, &mem_usage);
			if (ret < 0)
				break;
			else if (ret > 0)
				best_base = other_idx;
		}
```

**File:** builtin/pack-objects.c (L3035-3045)

```c
		if (DELTA(entry)) {
			struct unpacked swap = array[best_base];
			int dist = (window + idx - best_base) % window;
			int dst = best_base;
			while (dist--) {
				int src = (dst + 1) % window;
				array[dst] = array[src];
				dst = src;
			}
			array[dst] = swap;
		}
```

**File:** builtin/pack-objects.c (L3149-3276)

```c
static void ll_find_deltas(struct object_entry **list, unsigned list_size,
			   int window, int depth, unsigned *processed)
{
	struct thread_params *p;
	int i, ret, active_threads = 0;

	init_threaded_search();

	if (delta_search_threads <= 1) {
		find_deltas(list, &list_size, window, depth, processed);
		cleanup_threaded_search();
		return;
	}
	if (progress > pack_to_stdout)
		fprintf_ln(stderr, _("Delta compression using up to %d threads"),
			   delta_search_threads);
	CALLOC_ARRAY(p, delta_search_threads);

	/* Partition the work amongst work threads. */
	for (i = 0; i < delta_search_threads; i++) {
		unsigned sub_size = list_size / (delta_search_threads - i);

		/* don't use too small segments or no deltas will be found */
		if (sub_size < 2*window && i+1 < delta_search_threads)
			sub_size = 0;

		p[i].window = window;
		p[i].depth = depth;
		p[i].processed = processed;
		p[i].working = 1;
		p[i].data_ready = 0;

		/* try to split chunks on "path" boundaries */
		while (sub_size && sub_size < list_size &&
		       list[sub_size]->hash &&
		       list[sub_size]->hash == list[sub_size-1]->hash)
			sub_size++;

		p[i].list = list;
		p[i].list_size = sub_size;
		p[i].remaining = sub_size;

		list += sub_size;
		list_size -= sub_size;
	}

	/* Start work threads. */
	for (i = 0; i < delta_search_threads; i++) {
		if (!p[i].list_size)
			continue;
		pthread_mutex_init(&p[i].mutex, NULL);
		pthread_cond_init(&p[i].cond, NULL);
		ret = pthread_create(&p[i].thread, NULL,
				     threaded_find_deltas, &p[i]);
		if (ret)
			die(_("unable to create thread: %s"), strerror(ret));
		active_threads++;
	}

	/*
	 * Now let's wait for work completion.  Each time a thread is done
	 * with its work, we steal half of the remaining work from the
	 * thread with the largest number of unprocessed objects and give
	 * it to that newly idle thread.  This ensure good load balancing
	 * until the remaining object list segments are simply too short
	 * to be worth splitting anymore.
	 */
	while (active_threads) {
		struct thread_params *target = NULL;
		struct thread_params *victim = NULL;
		unsigned sub_size = 0;

		progress_lock();
		for (;;) {
			for (i = 0; !target && i < delta_search_threads; i++)
				if (!p[i].working)
					target = &p[i];
			if (target)
				break;
			pthread_cond_wait(&progress_cond, &progress_mutex);
		}

		for (i = 0; i < delta_search_threads; i++)
			if (p[i].remaining > 2*window &&
			    (!victim || victim->remaining < p[i].remaining))
				victim = &p[i];
		if (victim) {
			sub_size = victim->remaining / 2;
			list = victim->list + victim->list_size - sub_size;
			while (sub_size && list[0]->hash &&
			       list[0]->hash == list[-1]->hash) {
				list++;
				sub_size--;
			}
			if (!sub_size) {
				/*
				 * It is possible for some "paths" to have
				 * so many objects that no hash boundary
				 * might be found.  Let's just steal the
				 * exact half in that case.
				 */
				sub_size = victim->remaining / 2;
				list -= sub_size;
			}
			target->list = list;
			victim->list_size -= sub_size;
			victim->remaining -= sub_size;
		}
		target->list_size = sub_size;
		target->remaining = sub_size;
		target->working = 1;
		progress_unlock();

		pthread_mutex_lock(&target->mutex);
		target->data_ready = 1;
		pthread_cond_signal(&target->cond);
		pthread_mutex_unlock(&target->mutex);

		if (!sub_size) {
			pthread_join(target->thread, NULL);
			pthread_cond_destroy(&target->cond);
			pthread_mutex_destroy(&target->mutex);
			active_threads--;
		}
	}
	cleanup_threaded_search();
	free(p);
}
```

**File:** builtin/pack-objects.c (L3324-3352)

```c
static int should_attempt_deltas(struct object_entry *entry)
{
	if (DELTA(entry))
		/* This happens if we decided to reuse existing
		 * delta from a pack. "reuse_delta &&" is implied.
		 */
		return 0;

	if (!entry->type_valid ||
	    oe_size_less_than(&to_pack, entry, 50))
		return 0;

	if (entry->no_try_delta)
		return 0;

	if (!entry->preferred_base) {
		if (oe_type(entry) < 0)
			die(_("unable to get type of object %s"),
				oid_to_hex(&entry->idx.oid));
	} else if (oe_type(entry) < 0) {
		/*
		 * This object is not found, but we
		 * don't have to include it anyway.
		 */
		return 0;
	}

	return 1;
}
```

**File:** builtin/pack-objects.c (L3440-3558)

```c
static void ll_find_deltas_by_region(struct object_entry *list,
				     struct packing_region *regions,
				     uint32_t start, uint32_t nr)
{
	struct thread_params *p;
	int i, ret, active_threads = 0;
	unsigned int processed = 0;
	uint32_t progress_nr;
	init_threaded_search();

	if (!nr)
		return;

	progress_nr =  regions[nr - 1].start + regions[nr - 1].nr;
	if (delta_search_threads <= 1) {
		find_deltas_by_region(list, regions, start, nr);
		cleanup_threaded_search();
		return;
	}

	if (progress > pack_to_stdout)
		fprintf_ln(stderr,
			   Q_("Path-based delta compression using up to %d thread",
			      "Path-based delta compression using up to %d threads",
			      delta_search_threads),
			   delta_search_threads);
	CALLOC_ARRAY(p, delta_search_threads);

	if (progress)
		progress_state = start_progress(the_repository,
						_("Compressing objects by path"),
						progress_nr);
	/* Partition the work amongst work threads. */
	for (i = 0; i < delta_search_threads; i++) {
		unsigned sub_size = nr / (delta_search_threads - i);

		p[i].window = window;
		p[i].depth = depth;
		p[i].processed = &processed;
		p[i].working = 1;
		p[i].data_ready = 0;

		p[i].regions = regions;
		p[i].list_size = sub_size;
		p[i].remaining = sub_size;

		regions += sub_size;
		nr -= sub_size;
	}

	/* Start work threads. */
	for (i = 0; i < delta_search_threads; i++) {
		if (!p[i].list_size)
			continue;
		pthread_mutex_init(&p[i].mutex, NULL);
		pthread_cond_init(&p[i].cond, NULL);
		ret = pthread_create(&p[i].thread, NULL,
				     threaded_find_deltas_by_path, &p[i]);
		if (ret)
			die(_("unable to create thread: %s"), strerror(ret));
		active_threads++;
	}

	/*
	 * Now let's wait for work completion.  Each time a thread is done
	 * with its work, we steal half of the remaining work from the
	 * thread with the largest number of unprocessed objects and give
	 * it to that newly idle thread.  This ensure good load balancing
	 * until the remaining object list segments are simply too short
	 * to be worth splitting anymore.
	 */
	while (active_threads) {
		struct thread_params *target = NULL;
		struct thread_params *victim = NULL;
		unsigned sub_size = 0;

		progress_lock();
		for (;;) {
			for (i = 0; !target && i < delta_search_threads; i++)
				if (!p[i].working)
					target = &p[i];
			if (target)
				break;
			pthread_cond_wait(&progress_cond, &progress_mutex);
		}

		for (i = 0; i < delta_search_threads; i++)
			if (p[i].remaining > 2*window &&
			    (!victim || victim->remaining < p[i].remaining))
				victim = &p[i];
		if (victim) {
			sub_size = victim->remaining / 2;
			target->regions = victim->regions + victim->remaining - sub_size;
			victim->list_size -= sub_size;
			victim->remaining -= sub_size;
		}
		target->list_size = sub_size;
		target->remaining = sub_size;
		target->working = 1;
		progress_unlock();

		pthread_mutex_lock(&target->mutex);
		target->data_ready = 1;
		pthread_cond_signal(&target->cond);
		pthread_mutex_unlock(&target->mutex);

		if (!sub_size) {
			pthread_join(target->thread, NULL);
			pthread_cond_destroy(&target->cond);
			pthread_mutex_destroy(&target->mutex);
			active_threads--;
		}
	}
	cleanup_threaded_search();
	free(p);

	display_progress(progress_state, progress_nr);
	stop_progress(&progress_state);
}
```

**File:** builtin/pack-objects.c (L3560-3617)

```c
static void prepare_pack(int window, int depth)
{
	struct object_entry **delta_list;
	uint32_t i, nr_deltas;
	unsigned n;

	if (use_delta_islands)
		resolve_tree_islands(the_repository, progress, &to_pack);

	get_object_details();

	/*
	 * If we're locally repacking then we need to be doubly careful
	 * from now on in order to make sure no stealth corruption gets
	 * propagated to the new pack.  Clients receiving streamed packs
	 * should validate everything they get anyway so no need to incur
	 * the additional cost here in that case.
	 */
	if (!pack_to_stdout)
		do_check_packed_object_crc = 1;

	if (!to_pack.nr_objects || !window || !depth)
		return;

	if (path_walk)
		ll_find_deltas_by_region(to_pack.objects, to_pack.regions,
					 0, to_pack.nr_regions);

	ALLOC_ARRAY(delta_list, to_pack.nr_objects);
	nr_deltas = n = 0;

	for (i = 0; i < to_pack.nr_objects; i++) {
		struct object_entry *entry = to_pack.objects + i;

		if (!should_attempt_deltas(entry))
			continue;

		if (!entry->preferred_base)
			nr_deltas++;

		delta_list[n++] = entry;
	}

	if (nr_deltas && n > 1) {
		unsigned nr_done = 0;

		if (progress)
			progress_state = start_progress(the_repository,
							_("Compressing objects"),
							nr_deltas);
		QSORT(delta_list, n, type_size_sort);
		ll_find_deltas(delta_list, n, window+1, depth, &nr_done);
		stop_progress(&progress_state);
		if (nr_done != nr_deltas)
			die(_("inconsistency with delta count"));
	}
	free(delta_list);
}
```

### try_delta

# 增量尝试过程的逻辑

增量尝试过程是 Git pack-objects 中 `try_delta` 函数的核心逻辑，用于在两个对象间寻找最优的增量压缩关系。

## 核心流程

### 1. 基础验证

`try_delta` 函数首先进行基础验证 [1](#3-0) ：

```c
/* Don't bother doing diffs between different types */
if (oe_type(trg_entry) != oe_type(src_entry))
    return -1;
```

只有相同类型的对象才能进行增量压缩。

### 2. 重用增量检查

函数检查是否应该重用现有增量 [2](#3-1) ：

```c
if (reuse_delta && IN_PACK(trg_entry) &&
    IN_PACK(trg_entry) == IN_PACK(src_entry) &&
    !src_entry->preferred_base &&
    trg_entry->in_pack_type != OBJ_REF_DELTA &&
    trg_entry->in_pack_type != OBJ_OFS_DELTA)
    return 0;
```

### 3. 深度限制验证

检查增量链深度是否超出限制 [3](#3-2) ：

```c
/* Let's not bust the allowed depth. */
if (src->depth >= max_depth)
    return 0;
```

### 4. 大小过滤启发式

应用多种大小相关的过滤条件 [4](#3-3) ：

- 计算最大允许的增量大小
- 检查源对象和目标对象的大小差异
- 确保目标对象不会太小（小于源对象的 1/32）

### 5. 增量岛检查

如果启用了增量岛功能，确保两个对象在同一个岛内 [5](#3-4) 。

## 增量计算过程

### 1. 数据加载

如果对象数据尚未加载，从对象数据库读取 [6](#3-5) ：

```c
/* Load data if not already done */
if (!trg->data) {
    trg->data = odb_read_object(the_repository->objects,
                                &trg_entry->idx.oid, &type, &sz);
}
```

### 2. 增量索引创建

为源对象创建增量索引以加速增量计算 [7](#3-6) ：

```c
if (!src->index) {
    src->index = create_delta_index(src->data, src_size);
    if (!src->index) {
        warning(_("suboptimal pack - out of memory"));
        return 0;
    }
    *mem_usage += sizeof_delta_index(src->index);
}
```

### 3. 增量生成

使用索引计算实际的增量数据 [8](#3-7) ：

```c
delta_buf = create_delta(src->index, trg->data, trg_size, &delta_size, max_size);
if (!delta_buf)
    return 0;
```

## 增量质量评估

### 1. 现有增量比较

如果目标对象已有增量，比较新旧增量的质量 [9](#3-8) ：

```c
if (DELTA(trg_entry)) {
    /* Prefer only shallower same-sized deltas. */
    if (delta_size == DELTA_SIZE(trg_entry) &&
        src->depth + 1 >= trg->depth) {
        free(delta_buf);
        return 0;
    }
}
```

### 2. 增量缓存决策

根据增量大小决定是否缓存增量数据 [10](#3-9) ：

```c
if (delta_cacheable(src_size, trg_size, delta_size)) {
    delta_cache_size += delta_size;
    cache_unlock();
    trg_entry->delta_data = xrealloc(delta_buf, delta_size);
} else {
    cache_unlock();
    free(delta_buf);
}
```

## 增量关系建立

成功创建增量后，建立增量关系 [11](#3-10) ：

```c
SET_DELTA(trg_entry, src_entry);
SET_DELTA_SIZE(trg_entry, delta_size);
trg->depth = src->depth + 1;
```

## 返回值语义

- **返回 -1**：遇到不兼容的对象类型，停止尝试更多增量基础
- **返回 0**：增量尝试失败或不满足条件，可以继续尝试其他对象
- **返回 1**：成功创建增量，找到有效的增量基础

## Notes

- 增量尝试过程综合考虑了类型、大小、深度、内存使用等多个因素
- 通过增量索引技术大幅提升增量计算性能
- 智能的缓存策略平衡了内存使用和计算效率
- 严格的过滤条件避免无效的增量计算，提升整体性能

Wiki pages you might want to explore:

- [Git Overview (git/git)](/wiki/git/git#1)

### Citations

**File:** builtin/pack-objects.c (L2741-2753)

```c
static int try_delta(struct unpacked *trg, struct unpacked *src,
		     unsigned max_depth, unsigned long *mem_usage)
{
	struct object_entry *trg_entry = trg->entry;
	struct object_entry *src_entry = src->entry;
	unsigned long trg_size, src_size, delta_size, sizediff, max_size, sz;
	unsigned ref_depth;
	enum object_type type;
	void *delta_buf;

	/* Don't bother doing diffs between different types */
	if (oe_type(trg_entry) != oe_type(src_entry))
		return -1;
```

**File:** builtin/pack-objects.c (L2763-2768)

```c
	if (reuse_delta && IN_PACK(trg_entry) &&
	    IN_PACK(trg_entry) == IN_PACK(src_entry) &&
	    !src_entry->preferred_base &&
	    trg_entry->in_pack_type != OBJ_REF_DELTA &&
	    trg_entry->in_pack_type != OBJ_OFS_DELTA)
		return 0;
```

**File:** builtin/pack-objects.c (L2770-2772)

```c
	/* Let's not bust the allowed depth. */
	if (src->depth >= max_depth)
		return 0;
```

**File:** builtin/pack-objects.c (L2774-2792)

```c
	/* Now some size filtering heuristics. */
	trg_size = SIZE(trg_entry);
	if (!DELTA(trg_entry)) {
		max_size = trg_size/2 - the_hash_algo->rawsz;
		ref_depth = 1;
	} else {
		max_size = DELTA_SIZE(trg_entry);
		ref_depth = trg->depth;
	}
	max_size = (uint64_t)max_size * (max_depth - src->depth) /
						(max_depth - ref_depth + 1);
	if (max_size == 0)
		return 0;
	src_size = SIZE(src_entry);
	sizediff = src_size < trg_size ? trg_size - src_size : 0;
	if (sizediff >= max_size)
		return 0;
	if (trg_size < src_size / 32)
		return 0;
```

**File:** builtin/pack-objects.c (L2794-2795)

```c
	if (!in_same_island(&trg->entry->idx.oid, &src->entry->idx.oid))
		return 0;
```

**File:** builtin/pack-objects.c (L2797-2841)

```c
	/* Load data if not already done */
	if (!trg->data) {
		packing_data_lock(&to_pack);
		trg->data = odb_read_object(the_repository->objects,
					    &trg_entry->idx.oid, &type,
					    &sz);
		packing_data_unlock(&to_pack);
		if (!trg->data)
			die(_("object %s cannot be read"),
			    oid_to_hex(&trg_entry->idx.oid));
		if (sz != trg_size)
			die(_("object %s inconsistent object length (%"PRIuMAX" vs %"PRIuMAX")"),
			    oid_to_hex(&trg_entry->idx.oid), (uintmax_t)sz,
			    (uintmax_t)trg_size);
		*mem_usage += sz;
	}
	if (!src->data) {
		packing_data_lock(&to_pack);
		src->data = odb_read_object(the_repository->objects,
					    &src_entry->idx.oid, &type,
					    &sz);
		packing_data_unlock(&to_pack);
		if (!src->data) {
			if (src_entry->preferred_base) {
				static int warned = 0;
				if (!warned++)
					warning(_("object %s cannot be read"),
						oid_to_hex(&src_entry->idx.oid));
				/*
				 * Those objects are not included in the
				 * resulting pack.  Be resilient and ignore
				 * them if they can't be read, in case the
				 * pack could be created nevertheless.
				 */
				return 0;
			}
			die(_("object %s cannot be read"),
			    oid_to_hex(&src_entry->idx.oid));
		}
		if (sz != src_size)
			die(_("object %s inconsistent object length (%"PRIuMAX" vs %"PRIuMAX")"),
			    oid_to_hex(&src_entry->idx.oid), (uintmax_t)sz,
			    (uintmax_t)src_size);
		*mem_usage += sz;
	}
```

**File:** builtin/pack-objects.c (L2842-2851)

```c
	if (!src->index) {
		src->index = create_delta_index(src->data, src_size);
		if (!src->index) {
			static int warned = 0;
			if (!warned++)
				warning(_("suboptimal pack - out of memory"));
			return 0;
		}
		*mem_usage += sizeof_delta_index(src->index);
	}
```

**File:** builtin/pack-objects.c (L2853-2855)

```c
	delta_buf = create_delta(src->index, trg->data, trg_size, &delta_size, max_size);
	if (!delta_buf)
		return 0;
```

**File:** builtin/pack-objects.c (L2857-2864)

```c
	if (DELTA(trg_entry)) {
		/* Prefer only shallower same-sized deltas. */
		if (delta_size == DELTA_SIZE(trg_entry) &&
		    src->depth + 1 >= trg->depth) {
			free(delta_buf);
			return 0;
		}
	}
```

**File:** builtin/pack-objects.c (L2877-2884)

```c
	if (delta_cacheable(src_size, trg_size, delta_size)) {
		delta_cache_size += delta_size;
		cache_unlock();
		trg_entry->delta_data = xrealloc(delta_buf, delta_size);
	} else {
		cache_unlock();
		free(delta_buf);
	}
```

**File:** builtin/pack-objects.c (L2886-2890)

```c
	SET_DELTA(trg_entry, src_entry);
	SET_DELTA_SIZE(trg_entry, delta_size);
	trg->depth = src->depth + 1;

	return 1;
```

## diff-delta.c

### create_delta_index & create_delta

# create_delta_index 和 create_delta 原理分析

`create_delta_index` 和 `create_delta` 是 Git 增量压缩系统的核心组件，实现了高效的二进制差分算法。

## create_delta_index 原理

### 数据结构设计

`create_delta_index` 创建一个哈希表来索引源对象中的所有可能匹配位置 [1](#4-0) ：

```c
struct delta_index {
    unsigned long memsize;
    const void *src_buf;
    unsigned long src_size;
    unsigned int hash_mask;
    struct index_entry *hash[FLEX_ARRAY];
};
```

### Rabin 滚动哈希算法

函数使用 Rabin 滚动哈希算法来识别重复模式 [2](#4-1) ：

1. **窗口大小**：使用 `RABIN_WINDOW` 常量定义的固定窗口
2. **哈希计算**：通过滑动窗口计算每个位置的哈希值
3. **索引构建**：将哈希值映射到源对象中的位置

### 哈希冲突处理

为防止性能退化，算法限制每个哈希桶的条目数量 [3](#4-2) ：

- 使用 `HASH_LIMIT` 控制最大条目数
- 超出限制时均匀删除条目以保持分布

## create_delta 原理

### 增量指令格式

`create_delta` 生成两种指令来重建目标对象 [4](#4-3) ：

1. **复制指令**：从源对象复制字节范围
2. **插入指令**：插入新的字节数据

### 匹配搜索过程

算法在目标对象中滑动窗口，寻找源对象中的最佳匹配 [5](#4-4) ：

```c
for (entry = index->hash[i]; entry < index->hash[i+1]; entry++) {
    const unsigned char *ref = entry->ptr;
    const unsigned char *src = data;
    // 逐字节比较以找到最长匹配
    while (ref_size-- && *src++ == *ref)
        ref++;
}
```

### 指令优化策略

- **最小匹配长度**：只有匹配长度 ≥ 4 字节才使用复制指令
- **回溯优化**：尝试向后扩展匹配以获得更好的压缩效果 [6](#4-5)
- **大小编码**：使用变长编码减少指令开销

## 在 pack-objects 中的使用

这两个函数在 `try_delta` 中被调用来计算对象间的增量 [7](#4-6) ：

```c
if (!src->index) {
    src->index = create_delta_index(src->data, src_size);
}
delta_buf = create_delta(src->index, trg->data, trg_size, &delta_size, max_size);
```

## 性能优化特性

1. **内存效率**：索引结构紧凑，支持快速查找
2. **时间复杂度**：平均情况下接近线性时间
3. **压缩质量**：通过启发式规则平衡压缩率和速度

## Notes

- 这两个函数实现了 Git 特定的增量格式，与标准 diff 算法不同
- Rabin 哈希提供了良好的分布特性，适合二进制数据
- 算法设计考虑了 Git 对象的特点，如小文件多、相似性高等

Wiki pages you might want to explore:

- [Git Overview (git/git)](/wiki/git/git#1)

### Citations

**File:** diff-delta.c (L127-133)

```c
struct delta_index {
	unsigned long memsize;
	const void *src_buf;
	unsigned long src_size;
	unsigned int hash_mask;
	struct index_entry *hash[FLEX_ARRAY];
};
```

**File:** diff-delta.c (L148-204)

```c
	/* Determine index hash size.  Note that indexing skips the
	   first byte to allow for optimizing the Rabin's polynomial
	   initialization in create_delta(). */
	entries = (bufsize - 1) / RABIN_WINDOW;
	if (bufsize >= 0xffffffffUL) {
		/*
		 * Current delta format can't encode offsets into
		 * reference buffer with more than 32 bits.
		 */
		entries = 0xfffffffeU / RABIN_WINDOW;
	}
	hsize = entries / 4;
	for (i = 4; (1u << i) < hsize; i++);
	hsize = 1 << i;
	hmask = hsize - 1;

	/* allocate lookup index */
	memsize = sizeof(*hash) * hsize +
		  sizeof(*entry) * entries;
	mem = malloc(memsize);
	if (!mem)
		return NULL;
	hash = mem;
	mem = hash + hsize;
	entry = mem;

	MEMZERO_ARRAY(hash, hsize);

	/* allocate an array to count hash entries */
	hash_count = calloc(hsize, sizeof(*hash_count));
	if (!hash_count) {
		free(hash);
		return NULL;
	}

	/* then populate the index */
	prev_val = ~0;
	for (data = buffer + entries * RABIN_WINDOW - RABIN_WINDOW;
	     data >= buffer;
	     data -= RABIN_WINDOW) {
		unsigned int val = 0;
		for (i = 1; i <= RABIN_WINDOW; i++)
			val = ((val << 8) | data[i]) ^ T[val >> RABIN_SHIFT];
		if (val == prev_val) {
			/* keep the lowest of consecutive identical blocks */
			entry[-1].entry.ptr = data + RABIN_WINDOW;
			--entries;
		} else {
			prev_val = val;
			i = val & hmask;
			entry->entry.ptr = data + RABIN_WINDOW;
			entry->entry.val = val;
			entry->next = hash[i];
			hash[i] = entry++;
			hash_count[i]++;
		}
	}
```

**File:** diff-delta.c (L218-254)

```c
	for (i = 0; i < hsize; i++) {
		int acc;

		if (hash_count[i] <= HASH_LIMIT)
			continue;

		/* We leave exactly HASH_LIMIT entries in the bucket */
		entries -= hash_count[i] - HASH_LIMIT;

		entry = hash[i];
		acc = 0;

		/*
		 * Assume that this loop is gone through exactly
		 * HASH_LIMIT times and is entered and left with
		 * acc==0.  So the first statement in the loop
		 * contributes (hash_count[i]-HASH_LIMIT)*HASH_LIMIT
		 * to the accumulator, and the inner loop consequently
		 * is run (hash_count[i]-HASH_LIMIT) times, removing
		 * one element from the list each time.  Since acc
		 * balances out to 0 at the final run, the inner loop
		 * body can't be left with entry==NULL.  So we indeed
		 * encounter entry==NULL in the outer loop only.
		 */
		do {
			acc += hash_count[i] - HASH_LIMIT;
			if (acc > 0) {
				struct unpacked_index_entry *keep = entry;
				do {
					entry = entry->next;
					acc -= HASH_LIMIT;
				} while (acc > 0);
				keep->next = entry->next;
			}
			entry = entry->next;
		} while (entry);
	}
```

**File:** diff-delta.c (L375-401)

```c
	while (data < top) {
		if (msize < 4096) {
			struct index_entry *entry;
			val ^= U[data[-RABIN_WINDOW]];
			val = ((val << 8) | *data) ^ T[val >> RABIN_SHIFT];
			i = val & index->hash_mask;
			for (entry = index->hash[i]; entry < index->hash[i+1]; entry++) {
				const unsigned char *ref = entry->ptr;
				const unsigned char *src = data;
				unsigned int ref_size = ref_top - ref;
				if (entry->val != val)
					continue;
				if (ref_size > top - src)
					ref_size = top - src;
				if (ref_size <= msize)
					break;
				while (ref_size-- && *src++ == *ref)
					ref++;
				if (msize < ref - entry->ptr) {
					/* this is our best match so far */
					msize = ref - entry->ptr;
					moff = entry->ptr - ref_data;
					if (msize >= 4096) /* good enough */
						break;
				}
			}
		}
```

**File:** diff-delta.c (L418-432)

```c
				while (moff && ref_data[moff-1] == data[-1]) {
					/* we can match one byte back */
					msize++;
					moff--;
					data--;
					outpos--;
					if (--inscnt)
						continue;
					outpos--;  /* remove count slot */
					inscnt--;  /* make it -1 */
					break;
				}
				out[outpos - inscnt - 1] = inscnt;
				inscnt = 0;
			}
```

**File:** Documentation/gitformat-pack.adoc (L147-180)

```text
==== Instruction to copy from base object

  +----------+---------+---------+---------+---------+-------+-------+-------+
  | 1xxxxxxx | offset1 | offset2 | offset3 | offset4 | size1 | size2 | size3 |
  +----------+---------+---------+---------+---------+-------+-------+-------+

This is the instruction format to copy a byte range from the source
object. It encodes the offset to copy from and the number of bytes to
copy. Offset and size are in little-endian order.

All offset and size bytes are optional. This is to reduce the
instruction size when encoding small offsets or sizes. The first seven
bits in the first octet determine which of the next seven octets is
present. If bit zero is set, offset1 is present. If bit one is set
offset2 is present and so on.

Note that a more compact instruction does not change offset and size
encoding. For example, if only offset2 is omitted like below, offset3
still contains bits 16-23. It does not become offset2 and contains
bits 8-15 even if it's right next to offset1.

  +----------+---------+---------+
  | 10000101 | offset1 | offset3 |
  +----------+---------+---------+

In its most compact form, this instruction only takes up one byte
(0x80) with both offset and size omitted, which will have default
values zero. There is another exception: size zero is automatically
converted to 0x10000.

==== Instruction to add new data

  +----------+============+
  | 0xxxxxxx |    data    |
```

**File:** builtin/pack-objects.c (L2842-2855)

```c
	if (!src->index) {
		src->index = create_delta_index(src->data, src_size);
		if (!src->index) {
			static int warned = 0;
			if (!warned++)
				warning(_("suboptimal pack - out of memory"));
			return 0;
		}
		*mem_usage += sizeof_delta_index(src->index);
	}

	delta_buf = create_delta(src->index, trg->data, trg_size, &delta_size, max_size);
	if (!delta_buf)
		return 0;
```

Rabin 滚动哈希（Rabin Rolling Hash）是一种在处理长字符串或大数据流时极其高效的哈希技术。它是 **Rabin-Karp 字符串搜索算法**的核心，也被广泛应用于现代分布式系统和文件同步工具中。

以下是 Rabin 滚动哈希的详细介绍：

### 一、 工作原理

滚动哈希的核心思想是：**利用“滑动窗口”内前一个状态的哈希值，在 $O(1)$ 的时间内推导出下一个状态的哈希值**，而无需重新遍历整个窗口的内容。

#### 1. 数学模型（多项式哈希）

它通常将字符串视为一个高进制的数字。假设字符串由字符 $c_1, c_2, \dots, c_k$ 组成，我们选择一个基数 $B$（通常是一个质数，如 31 或 257）和一个模数 $M$（通常是一个很大的质数，以减少冲突）。

哈希值的计算公式为：
$$H = (c_1 \cdot B^{k-1} + c_2 \cdot B^{k-2} + \dots + c_k \cdot B^0) \mod M$$

#### 2. “滚动”过程

当窗口向右滑动一格，移出字符 $S_{old}$，移入字符 $S_{new}$ 时，新的哈希值 $H_{next}$ 可以通过以下三步计算得出：

1. **去首**：减去移出字符的贡献：$H' = H - (S_{old} \cdot B^{k-1})$
2. **左移**：整体乘以基数 $B$（相当于进位）：$H'' = H' \cdot B$
3. **加末**：加上新字符的贡献：$H_{next} = (H'' + S_{new}) \mod M$

整个推导过程只涉及常数次加减乘运算，因此时间复杂度是 **$O(1)$**。

---

### 二、 适用场景

Rabin 滚动哈希非常适合需要在海量数据中寻找“局部重复”或“特定模式”的情况：

#### 1. 字符串模式匹配（Rabin-Karp 算法）

这是最经典的应用。在一段文本中搜索某个关键词。相比于 KMP 算法，Rabin-Karp 的优势在于**多模式匹配**：如果你要同时搜索 100 个关键词，只需预先算出这 100 个词的哈希值存入集合，然后在文本上滚动一次哈希，对比集合即可。

#### 2. 数据去重与文件同步（rsync）

著名的文件同步工具 **rsync** 使用了类似的滚动哈希思想。它将文件切分成块，通过滚动哈希快速找到两个大文件之间相同的数据块，从而只传输发生变化的部分（增量传输），极大地节省了带宽。

#### 3. 内容识别与防剽窃

在检测文档抄袭或代码重复时，系统会将文档切成固定长度的小段（Shingles），利用滚动哈希生成指纹。如果两个文档中有大量相同的哈希值，则判定存在重合。

#### 4. 生物信息学

在 DNA 序列分析中，需要在极其庞大的基因序列中寻找特定的碱基序列模式（如 A-T-C-G 的组合），滚动哈希能以线性时间完成这类大规模搜索。

#### 5. 内容定界分块（Content-Defined Chunking, CDC）

在云存储（如网盘）的后端，为了实现秒传功能，需要对文件去重。使用滚动哈希（如 Rabin Fingerprint）可以动态决定文件的分块边界，使得即使在文件开头插入一个字节，后续大部分数据块的哈希值依然保持不变。

---

### 三、 算法优缺点

| 优点                                                     | 缺点                                                                         |
| :------------------------------------------------------- | :--------------------------------------------------------------------------- |
| **高效性**：窗口移动时计算哈希的代价极小。               | **哈希冲突**：不同的字符串可能产生相同的哈希值（Spurious Hit）。             |
| **易扩展**：天然支持多模式搜索。                         | **需二次验证**：当哈希值匹配时，通常需要再进行一次字符比对以确保准确。       |
| **流式处理**：不需要预知整个字符串，适合处理在线流数据。 | **最差性能**：如果模数选得不好导致冲突频繁，复杂度会退化到 $O(n \times m)$。 |

### 四、 总结

Rabin 滚动哈希是一种“**以空间和微弱随机风险换取极端时间效率**”的策略。它的美感在于将复杂的字符串比较转化为了简单的算术运算，是处理海量文本和大规模数据同步不可或缺的利器。
