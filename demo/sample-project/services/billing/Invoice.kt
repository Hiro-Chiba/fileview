package dev.nebula.billing

import java.math.BigDecimal

data class Invoice(
    val customer: String,
    val cpuSeconds: Long,
    val storageGb: Double,
) {
    fun total(): BigDecimal {
        val cpu = BigDecimal(cpuSeconds) * CPU_RATE
        val storage = BigDecimal(storageGb) * STORAGE_RATE
        return (cpu + storage).setScale(2, java.math.RoundingMode.HALF_UP)
    }

    companion object {
        val CPU_RATE = BigDecimal("0.00002")
        val STORAGE_RATE = BigDecimal("0.023")
    }
}
