type PpmOrgCustomerTrace struct {
    Id int64 `gorm:"column:id;type:bigint(20);comment:主键" json:"id" form:"id"`
    {{field_arr}}
}
