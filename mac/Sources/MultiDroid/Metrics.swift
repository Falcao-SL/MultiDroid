import Darwin
import Foundation

/// Memória e CPU lidas do kernel, sem processo auxiliar: `proc_pid_rusage`
/// dá o mesmo "physical footprint" que o Monitor de Atividade e o vmmap.
enum Metrics {
    static let gb = 1_073_741_824.0

    private static let timebase: Double = {
        var tb = mach_timebase_info_data_t()
        mach_timebase_info(&tb)
        return Double(tb.numer) / Double(tb.denom)
    }()

    static func usage(_ pid: pid_t) -> (footprint: UInt64, cpuNanos: UInt64)? {
        var info = rusage_info_v4()
        let r = withUnsafeMutablePointer(to: &info) { ptr in
            ptr.withMemoryRebound(to: rusage_info_t?.self, capacity: 1) { proc_pid_rusage(pid, RUSAGE_INFO_V4, $0) }
        }
        guard r == 0 else { return nil }
        let cpu = Double(info.ri_user_time + info.ri_system_time) * timebase
        return (info.ri_phys_footprint, UInt64(cpu))
    }

    static var physicalGB: Double { Double(ProcessInfo.processInfo.physicalMemory) / gb }

    /// Memória disponível do Mac: livre + inativa + especulativa + purgeável.
    static func availableGB() -> Double {
        var stats = vm_statistics64()
        var count = mach_msg_type_number_t(MemoryLayout<vm_statistics64>.size / MemoryLayout<integer_t>.size)
        let r = withUnsafeMutablePointer(to: &stats) { ptr in
            ptr.withMemoryRebound(to: integer_t.self, capacity: Int(count)) {
                host_statistics64(mach_host_self(), HOST_VM_INFO64, $0, &count)
            }
        }
        guard r == KERN_SUCCESS else { return 0 }
        let pages = UInt64(stats.free_count) + UInt64(stats.inactive_count) + UInt64(stats.speculative_count) + UInt64(stats.purgeable_count)
        return Double(pages * UInt64(getpagesize())) / gb
    }

    static func swapUsedGB() -> Double {
        var sw = xsw_usage()
        var size = MemoryLayout<xsw_usage>.size
        guard sysctlbyname("vm.swapusage", &sw, &size, nil, 0) == 0 else { return 0 }
        return Double(sw.xsu_used) / gb
    }
}
