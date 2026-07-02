回看了debug过程中修改的所有问题，记录在该文档中

1. ‘dispatch_syscall' 在 'SYS_CLOCK_GETTIME' 中使用了未定义的常量 'BOOT_EPOCH'。  
该常量表示启动时刻的基准偏移量，补上了一个默认设为 0 的常量。

2. 修复了一堆类型未对齐的问题，略

3. 修复了一堆未明确类型的问题，略

4.  'reg_class' 没有写输出，补上了，还有其他的缺输出/类型不对，略

5. 'Kernel' 访问了 'self.disk' 但是自己并不拥有这个字段，补上了。
'Disk' 在 chaos 中模拟了磁盘，对于最高级的结构 kernel 来说由它管理磁盘资源是正确的。

6. 修复了一堆调用接口时参数不匹配的问题，略

7. 给一些 option 加了fallback

8. 'group_01.rs' 'basic_bkl_double_acquire_single_release' 这个点里同一个 owner 进两次锁之后退一次，期望行为是 还拥有锁
在 'GKL.leave()' 里改了只修改 'depth'

9. 'group_01.rs' 'basic_cross_module_lock_order' 这里考虑这样一件事，’FramePool‘ 有保护自己数据的mutex，因此它不应该被 'GKL' 这把全局大锁控制，修改了 ‘FramePool::get()' 不强制要求拿 GKL 锁。但是整体来看，可能还是需要某种更高权限的锁的。

10. 'group_02.rs' 'basic_sleep_under_spinlock_uniprocessor()' 检查了channel的recv功能在channel为空的情况下的运行，正常情况应该是，channel在释放guard锁之后park自己当前线程。

11. 'group_03.rs'  'basic_condvar_signal_before_wait()' 对于 syncqueue 的理解：当syncqueue中没有park的thread的时候收到signal，正确的行为不是直接忽略，而是记录下这个credit，当有线程准备park的时候消费这个credit。

12. ‘group_03.rs’ ‘basic_spurious_wakeup_no_recheck()‘ 这里原来syncqueue中被park 的线程在被唤醒之后直接返回true，但实际的期望行为是当它被唤醒的时候应该再去检查它的predicate的真实结果并返回。syncqueue本身只是托管线程，等待的‘某个状态变成我需要的样子’需要parkon来解决。

13. ‘group_06.rs’ ‘basic_block_read_success()’ 这里很奇怪，要求在‘disk’中‘read_block' 读取成功之后就将buffer填充为0xAA，我姑且认为这是chaos的特殊设计

14. ‘group08' 'basic_ring_full_reject' 在circbuf中加入超过len的内容，应该直接报错，旧的逻辑在报错之前已经移动了writecursor

15. 'group09' 'basic_save_restore_context()' Context 应当在‘capture’时正确保存传入的reg，在‘apply’时正确恢复，原来代码中恶意交换了前两个寄存器。

16. ‘group09’ ‘basic_interrupt_mask_set’ ‘TrapCtl::configure’ trapctl 是用来管理触发trap时候的行为的，trap mask是用来控制哪些trap允许进入，之前代码中没有正确处理clear和setbits

17. ‘ basic_page_fault_in_process_context() ' 在‘TrapCtl::on_pgfault()' 中错误过滤了一个正常地址的pagefault 修改了判断返回错误的标准不超过kernelspace。
但是值得注意的是后面还有当前不active和nested的判断。个人理解是在一些情况下也是需要内核态的pagefault的，比如存在lazy-mapped的内核页等。目前只改到能过

18. group 10‘basic_access_ok_overflow’ 针对一个‘check_access',也就是访问区间是否overlap内核区域，处理了一个overflow...

19. group 11 basic_fork_exec_workload() 修复group1的问题之后自己好了

20. group11另一个点改了group10的问题之后自己好了

